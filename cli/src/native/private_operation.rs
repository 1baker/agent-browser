//! Internal parser for authenticated, encrypted staging payloads. Parsing is not
//! proof of consent, source provenance, live account identity, or DOM safety.
//! Secret-bearing types deliberately implement neither Debug nor Serialize.
//! The trusted executor must JSON-escape selector/value arguments, verify the
//! exact live identity and account selector, and check login form POST/action.
use serde::Deserialize;
use serde_json::Value;

const INVALID: &str = "private_operation_invalid";
const MAX_BYTES: usize = 64 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PrivateOperation {
    schema: String,
    service_tab_handle: Value,
    expected_origin: String,
    expected_url: String,
    consent_sha256: String,
    account_scope: String,
    operation: Operation,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Operation {
    Login {
        email_selector: String,
        password_selector: String,
        submit_selector: String,
        email: String,
        password: String,
    },
    BackupCode {
        code_selector: String,
        submit_selector: String,
        account_selector: String,
        account_value: String,
        code: String,
        source_scope: String,
    },
    Renew {
        submit_selector: String,
        account_selector: String,
        account_value: String,
    },
    ReadKey {
        selector: String,
        account_selector: String,
        account_value: String,
    },
}

fn selectors(values: &[&str]) -> Result<(), &'static str> {
    for (index, selector) in values.iter().enumerate() {
        if selector.is_empty()
            || selector.len() > 1024
            || selector.trim() != *selector
            || selector.chars().any(char::is_control)
            || values[..index].contains(selector)
        {
            return Err(INVALID);
        }
    }
    Ok(())
}

fn canonical_email(email: &str) -> bool {
    if email.len() > 254 || !email.is_ascii() || email != email.to_ascii_lowercase() {
        return false;
    }
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && local.len() <= 64
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !local.contains("..")
        && local.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || b".!#$%&'*+-/=?^_`{|}~".contains(&byte)
        })
        && domain.contains('.')
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

impl PrivateOperation {
    /// Fixed diagnostics only, including serde failures which may otherwise
    /// contain rejected secret values or field names. No input is logged.
    pub(crate) fn parse(payload: &[u8]) -> Result<Self, &'static str> {
        if payload.is_empty() || payload.len() > MAX_BYTES {
            return Err(INVALID);
        }
        let value: Self = serde_json::from_slice(payload).map_err(|_| INVALID)?;
        value.validate()?;
        Ok(value)
    }

    fn validate(&self) -> Result<(), &'static str> {
        if self.schema != "agent-browser.private-operation.v1"
            || self
                .service_tab_handle
                .as_object()
                .is_none_or(|handle| handle.is_empty())
            || self.consent_sha256.len() != 64
            || !self
                .consent_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(INVALID);
        }
        let account = self
            .account_scope
            .strip_prefix("login.gov:")
            .ok_or(INVALID)?;
        if !canonical_email(account) {
            return Err(INVALID);
        }
        let allowed_origin = match &self.operation {
            Operation::Login {
                email_selector,
                password_selector,
                submit_selector,
                email,
                password,
            } => {
                selectors(&[email_selector, password_selector, submit_selector])?;
                if email != account
                    || password.is_empty()
                    || password.len() > 4096
                    || password.contains('\0')
                {
                    return Err(INVALID);
                }
                "https://secure.login.gov"
            }
            Operation::BackupCode {
                code_selector,
                submit_selector,
                account_selector,
                account_value,
                code,
                source_scope,
            } => {
                selectors(&[code_selector, submit_selector, account_selector])?;
                if account_value != account
                    || code.is_empty()
                    || code.len() > 256
                    || !code
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b' ' | b'-'))
                    || !code.bytes().any(|byte| byte.is_ascii_alphanumeric())
                    || source_scope.is_empty()
                    || source_scope.len() > 2048
                    || source_scope.trim() != source_scope
                    || !source_scope
                        .bytes()
                        .all(|byte| (b' '..=b'~').contains(&byte))
                {
                    return Err(INVALID);
                }
                "https://secure.login.gov"
            }
            Operation::Renew {
                submit_selector,
                account_selector,
                account_value,
            } => {
                selectors(&[submit_selector, account_selector])?;
                if account_value != account {
                    return Err(INVALID);
                }
                "https://sam.gov"
            }
            Operation::ReadKey {
                selector,
                account_selector,
                account_value,
            } => {
                selectors(&[selector, account_selector])?;
                if account_value != account {
                    return Err(INVALID);
                }
                "https://sam.gov"
            }
        };
        if self.expected_origin != allowed_origin
            || self.expected_url.len() > 4096
            || self
                .expected_url
                .chars()
                .any(|value| value.is_control() || value.is_whitespace())
        {
            return Err(INVALID);
        }
        let url = url::Url::parse(&self.expected_url).map_err(|_| INVALID)?;
        if url.scheme() != "https"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port().is_some()
            || url.fragment().is_some()
            || url.origin().ascii_serialization() != allowed_origin
            || url.as_str() != self.expected_url
        {
            return Err(INVALID);
        }
        Ok(())
    }

    pub(crate) fn service_tab_handle(&self) -> &Value {
        &self.service_tab_handle
    }
    pub(crate) fn expected_origin(&self) -> &str {
        &self.expected_origin
    }
    pub(crate) fn expected_url(&self) -> &str {
        &self.expected_url
    }
    pub(crate) fn consent_sha256(&self) -> &str {
        &self.consent_sha256
    }
    pub(crate) fn account_scope(&self) -> &str {
        &self.account_scope
    }
    pub(crate) fn operation(&self) -> &Operation {
        &self.operation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn envelope(operation: Value, origin: &str) -> Value {
        json!({"schema":"agent-browser.private-operation.v1",
            "service_tab_handle":{"handleId":"synthetic-reviewed-handle"},
            "expected_origin":origin,"expected_url":format!("{origin}/synthetic-path"),
            "consent_sha256":"a".repeat(64),"account_scope":"login.gov:operator@example.org",
            "operation":operation})
    }
    fn login() -> Value {
        envelope(
            json!({"kind":"login","email_selector":"#email","password_selector":"#password",
            "submit_selector":"#submit","email":"operator@example.org","password":"SYNTHETIC_PASSWORD_SENTINEL"}),
            "https://secure.login.gov",
        )
    }
    fn parse(value: &Value) -> Result<PrivateOperation, &'static str> {
        PrivateOperation::parse(&serde_json::to_vec(value).unwrap())
    }
    fn rejected(value: &Value) {
        assert!(matches!(parse(value), Err("private_operation_invalid")));
    }

    #[test]
    fn all_four_typed_operations_are_accepted_with_exact_account_scope() {
        let login = parse(&login()).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(login.account_scope(), "login.gov:operator@example.org");
        assert_eq!(login.expected_origin(), "https://secure.login.gov");
        assert_eq!(
            login.expected_url(),
            "https://secure.login.gov/synthetic-path"
        );
        assert_eq!(login.consent_sha256(), "a".repeat(64));
        assert!(login.service_tab_handle().is_object());
        assert!(matches!(login.operation(), Operation::Login { .. }));
        for (operation, origin) in [
            (
                json!({"kind":"backup_code","code_selector":"#code","submit_selector":"#submit",
                "account_selector":"#account","account_value":"operator@example.org",
                "code":"ABCD-1234","source_scope":"slack:reviewed-message:slot-0"}),
                "https://secure.login.gov",
            ),
            (
                json!({"kind":"renew","submit_selector":"#renew","account_selector":"#account",
                "account_value":"operator@example.org"}),
                "https://sam.gov",
            ),
            (
                json!({"kind":"read_key","selector":"#key","account_selector":"#account",
                "account_value":"operator@example.org"}),
                "https://sam.gov",
            ),
        ] {
            assert!(parse(&envelope(operation, origin)).is_ok());
        }
    }

    #[test]
    fn malformed_extra_and_secret_bearing_errors_are_fixed() {
        for payload in [
            b"SYNTHETIC_PASSWORD_SENTINEL".as_slice(),
            b"{}",
            b"null",
            b"[]",
            b"{",
        ] {
            assert!(matches!(
                PrivateOperation::parse(payload),
                Err("private_operation_invalid")
            ));
        }
        assert!(PrivateOperation::parse(&vec![b' '; MAX_BYTES + 1]).is_err());
        let mut value = login();
        value["extra"] = json!("SYNTHETIC_PASSWORD_SENTINEL");
        rejected(&value);
        let mut value = login();
        value["operation"]["extra"] = json!("SYNTHETIC_PASSWORD_SENTINEL");
        rejected(&value);
        let mut value = login();
        value["operation"]["password"] = json!({"SYNTHETIC_PASSWORD_SENTINEL":true});
        rejected(&value);
        let error = match parse(&value) {
            Err(error) => error,
            Ok(_) => panic!("invalid payload accepted"),
        };
        assert_eq!(error, INVALID);
        assert!(!error.contains("SYNTHETIC_PASSWORD_SENTINEL"));
    }

    #[test]
    fn origin_url_and_operation_cross_scope_are_rejected() {
        for url in [
            "http://secure.login.gov/synthetic-path",
            "https://secure.login.gov.evil.example/path",
            "https://sam.gov/path",
            "https://user:password@secure.login.gov/path",
            "https://secure.login.gov:8443/path",
            "https://secure.login.gov/path#fragment",
            " https://secure.login.gov/path",
            "https://secure.login.gov:443/path",
        ] {
            let mut value = login();
            value["expected_url"] = json!(url);
            rejected(&value);
        }
        let mut value = login();
        value["expected_origin"] = json!("https://sam.gov");
        value["expected_url"] = json!("https://sam.gov/path");
        rejected(&value);
        rejected(&envelope(
            json!({"kind":"read_key","selector":"#key","account_selector":"#account",
            "account_value":"operator@example.org"}),
            "https://secure.login.gov",
        ));
    }

    #[test]
    fn account_binding_and_required_selector_proof_cannot_be_omitted() {
        for account in [
            "operator@example.net",
            "Operator@example.org",
            " operator@example.org",
            "a@b",
            "a@b..com",
        ] {
            let mut value = login();
            value["account_scope"] = json!(format!("login.gov:{account}"));
            rejected(&value);
        }
        let mut value = login();
        value["operation"]["email"] = json!("different@example.org");
        rejected(&value);
        rejected(&envelope(
            json!({"kind":"renew","submit_selector":"#renew"}),
            "https://sam.gov",
        ));
        rejected(&envelope(
            json!({"kind":"read_key","selector":"#key","account_selector":"#account",
            "account_value":"different@example.org"}),
            "https://sam.gov",
        ));
        let mut value = login();
        value["operation"]["email_selector"] = json!("#password");
        rejected(&value);
        let mut value = login();
        value["operation"]["submit_selector"] = json!("x".repeat(1025));
        rejected(&value);
    }

    #[test]
    fn backup_reservation_is_coupled_to_backup_operation_only() {
        let mut value = login();
        value["backup_reservation"] = json!({"source_scope":"slot","code":"SYNTHETICCODE"});
        rejected(&value);
        let mut value = login();
        value["operation"]["source_scope"] = json!("slot");
        rejected(&value);
        let operation = json!({"kind":"backup_code","code_selector":"#code","submit_selector":"#submit",
            "account_selector":"#account","account_value":"operator@example.org","code":"ABCD-1234"});
        rejected(&envelope(operation, "https://secure.login.gov"));
    }
}
