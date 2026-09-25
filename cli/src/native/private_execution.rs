//! Internal one-shot private DOM execution. No ordinary command envelope, raw
//! JavaScript input, plaintext result serialization or automatic retry is exposed.
//! This does not enable live execution: trusted consent/source ingestion, route
//! binding and multi-stage navigation/cleanup coordination remain required.

use super::actions::DaemonState;
use super::cdp::client::CdpClient;
use super::privacy_gate::PrivatePermit;
use super::private_attachment::PrivateAttachment;
use super::private_broker::revalidate_private_target;
use super::private_identity::top_level_origin_guard;
use super::private_operation::{Operation, PrivateOperation};
use super::private_secret_store::SecretStore;
use serde_json::{json, Value};
use std::time::Duration;

/// Fixed acknowledgements only. Neither variant proves authentication, renewal
/// success or consumer activation. The actual result remains encrypted in-store.
pub(crate) enum PrivateStepOutcome {
    DispatchAttempted,
    ResultStored,
}

/// Execute one authenticated staged operation under the existing exact permit.
/// Admission precedes all DOM mutation; failures and cancellation consume the
/// reference and retain privacy. The caller must not retry or try another code.
pub(crate) async fn execute_private_operation(
    state: &DaemonState,
    store: &SecretStore,
    reference: &str,
    permit: &PrivatePermit,
    attachment: &PrivateAttachment,
) -> Result<PrivateStepOutcome, &'static str> {
    let payload = store.load(reference)?;
    let operation = PrivateOperation::parse(&payload)?;
    revalidate_private_target(
        state,
        operation.service_tab_handle(),
        operation.expected_origin(),
        operation.expected_url(),
        permit,
    )
    .await?;
    let browser = state
        .browser
        .as_ref()
        .ok_or("private_browser_unavailable")?;
    let session = attachment.session_id(&browser.client, permit)?;
    execute_staged_dom(
        &browser.client,
        permit,
        session,
        store,
        reference,
        &operation,
    )
    .await
}

// Called only after daemon-owned locked revalidation. Kept separate to exercise
// actual receipt/transport/result ordering with synthetic CDP peers.
async fn execute_staged_dom(
    client: &CdpClient,
    permit: &PrivatePermit,
    session: &str,
    store: &SecretStore,
    reference: &str,
    operation: &PrivateOperation,
) -> Result<PrivateStepOutcome, &'static str> {
    if let Operation::BackupCode {
        code, source_scope, ..
    } = operation.operation()
    {
        store.reserve_backup_code(
            reference,
            operation.consent_sha256(),
            operation.account_scope(),
            source_scope,
            code.as_bytes(),
        )?;
    }
    store.admit(reference)?;
    let result = tokio::time::timeout(
        Duration::from_secs(12),
        execute_dom(client, permit, session, operation),
    )
    .await
    .map_err(|_| "private_execution_failed")??;
    let bytes = serde_json::to_vec(&result).map_err(|_| "private_execution_failed")?;
    store.store_result(reference, &bytes)?;
    Ok(
        if matches!(operation.operation(), Operation::ReadKey { .. }) {
            PrivateStepOutcome::ResultStored
        } else {
            PrivateStepOutcome::DispatchAttempted
        },
    )
}

async fn execute_dom(
    client: &CdpClient,
    permit: &PrivatePermit,
    session: &str,
    operation: &PrivateOperation,
) -> Result<Value, &'static str> {
    let timeout = Duration::from_secs(3);
    let tree = client
        .send_private_command(permit, "Page.getFrameTree", None, Some(session), timeout)
        .await?
        .into_value();
    let frame = tree
        .pointer("/frameTree/frame")
        .ok_or("private_execution_failed")?;
    if frame.get("parentId").is_some()
        || frame.get("url").and_then(Value::as_str) != Some(operation.expected_url())
    {
        return Err("private_execution_failed");
    }
    let frame_id = frame
        .get("id")
        .and_then(Value::as_str)
        .ok_or("private_execution_failed")?;
    let world = client
        .send_private_command(
            permit,
            "Page.createIsolatedWorld",
            Some(json!({"frameId":frame_id,"worldName":"agent-browser-private-operation"})),
            Some(session),
            timeout,
        )
        .await?
        .into_value();
    let context = world
        .get("executionContextId")
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
        .ok_or("private_execution_failed")?;
    let result = client
        .send_private_command(
            permit,
            "Runtime.evaluate",
            Some(json!({
                "contextId":context,"returnByValue":true,"awaitPromise":false,
                "expression":expression(operation)?,
            })),
            Some(session),
            timeout,
        )
        .await?
        .into_value();
    if result.get("exceptionDetails").is_some() {
        return Err("private_execution_failed");
    }
    let value = result
        .pointer("/result/value")
        .ok_or("private_execution_failed")?;
    if matches!(operation.operation(), Operation::ReadKey { .. }) {
        let key = value
            .get("key")
            .and_then(Value::as_str)
            .ok_or("private_execution_failed")?;
        if !(16..=4096).contains(&key.len())
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
            || value.as_object().is_none_or(|v| v.len() != 1)
        {
            return Err("private_execution_failed");
        }
    } else if *value != json!({"dispatch_attempted":true}) {
        return Err("private_execution_failed");
    }
    Ok(value.clone())
}

/// Only the private transport may carry this string; it contains secrets.
/// JSON encoding keeps selectors/values data, never executable source fragments.
fn expression(operation: &PrivateOperation) -> Result<String, &'static str> {
    let guard = top_level_origin_guard(operation.expected_origin(), operation.expected_url())?;
    let data = match operation.operation() {
        Operation::Login {
            email_selector,
            password_selector,
            submit_selector,
            email,
            password,
        } => {
            json!({"kind":"login","fields":[[email_selector,email],[password_selector,password]],"submit":submit_selector})
        }
        Operation::BackupCode {
            code_selector,
            submit_selector,
            account_selector,
            account_value,
            code,
            ..
        } => {
            json!({"kind":"backup","fields":[[code_selector,code]],"submit":submit_selector,"account":account_selector,"accountValue":account_value})
        }
        Operation::Renew {
            submit_selector,
            account_selector,
            account_value,
        } => {
            json!({"kind":"renew","submit":submit_selector,"account":account_selector,"accountValue":account_value})
        }
        Operation::ReadKey {
            selector,
            account_selector,
            account_value,
        } => {
            json!({"kind":"read","selector":selector,"account":account_selector,"accountValue":account_value})
        }
    };
    Ok(format!(
        r#"(() => {{
      const guard = () => {{ {guard} }};
      guard();
      const data = {data};
      const fail = () => {{ throw new Error('private_execution_failed'); }};
      const one = selector => {{
        const nodes = document.querySelectorAll(selector);
        if (nodes.length !== 1 || !nodes[0].isConnected) fail();
        return nodes[0];
      }};
      const visible = node => {{
        const style = getComputedStyle(node);
        if (!node.getClientRects().length || style.visibility !== 'visible' || style.display === 'none') fail();
      }};
      const account = () => {{
        if (data.account) {{
          const node = one(data.account);
          visible(node);
          const value = node instanceof HTMLInputElement ? node.value : node.textContent;
          if (value.trim() !== data.accountValue) fail();
        }}
      }};
      account();
      if (data.kind === 'read') {{
        const node = one(data.selector);
        const key = (node instanceof HTMLInputElement ? node.value : node.textContent).trim();
        if (!/^[A-Za-z0-9_-]{{16,4096}}$/.test(key)) fail();
        guard(); account();
        return {{key}};
      }}
      const button = one(data.submit);
      visible(button);
      if (!(button instanceof HTMLButtonElement || button instanceof HTMLInputElement) || button.disabled) fail();
      const form = button.form;
      const retainedTarget = () => {{
        const safe = target => !target || target.toLowerCase() === '_self';
        if ([...document.querySelectorAll('base[target]')].some(base => !safe(base.getAttribute('target')))) return false;
        return safe(button.hasAttribute('formtarget') ? button.formTarget : form?.target);
      }};
      const formSafe = () => button.type === 'submit' && button.form === form
        && (button.hasAttribute('formmethod') ? button.formMethod : form.method).toLowerCase() === 'post'
        && new URL(button.hasAttribute('formaction') ? button.formAction : form.action, location.href).origin === location.origin
        && retainedTarget();
      // Form-backed renewals obey the same destination restrictions. A reviewed
      // SPA renewal control must be a real non-submit button without a form.
      const submitSafe = () => form instanceof HTMLFormElement ? formSafe()
        : data.kind === 'renew' && !button.form && button.type === 'button' && retainedTarget();
      if (!submitSafe()) fail();
      if (data.kind !== 'renew') {{
        if (!(form instanceof HTMLFormElement) || !formSafe()) fail();
        const fields = data.fields.map(([selector,value]) => {{
          const node = one(selector);
          visible(node);
          if (!(node instanceof HTMLInputElement) || node.disabled || node.readOnly || node.form !== form) fail();
          return [node,value];
        }});
        if (new Set(fields.map(([node]) => node)).size !== fields.length) fail();
        if (data.kind === 'login' && (fields[1][0].type !== 'password'
            || !['email','text'].includes(fields[0][0].type))) fail();
        if (data.kind === 'backup' && !['text','tel','password'].includes(fields[0][0].type)) fail();
        const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set;
        for (const [node,value] of fields) {{
          guard(); account();
          if (!node.isConnected || node.form !== form) fail();
          setter.call(node,value);
          node.dispatchEvent(new Event('input',{{bubbles:true}}));
          node.dispatchEvent(new Event('change',{{bubbles:true}}));
        }}
        guard(); account();
        if (!formSafe()
            || fields.some(([node,value]) => !node.isConnected || node.form !== form || node.value !== value)) fail();
      }}
      guard(); account();
      if (!button.isConnected || button.disabled || !submitSafe()) fail();
      HTMLElement.prototype.click.call(button);
      return {{dispatch_attempted:true}};
    }})()"#
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::process::{Command, Stdio};

    fn login() -> PrivateOperation {
        operation(
            "https://secure.login.gov",
            json!({
                "kind":"login","email_selector":"#email","password_selector":"#password",
                "submit_selector":"#submit","email":"user@example.test",
                "password":"SYNTHETIC_SECRET_'_\\_\n_);throw new Error('injected')"
            }),
        )
    }

    fn operation(origin: &str, operation: Value) -> PrivateOperation {
        PrivateOperation::parse(
            &serde_json::to_vec(&json!({
                "schema":"agent-browser.private-operation.v1",
                "service_tab_handle":{
                    "profileId":"sam","browserId":"session:default","sessionName":"default",
                    "tabId":"tab","targetId":"retained","url":format!("{origin}/"),
                    "leaseId":"default","leaseState":"exclusive","leaseHeartbeatExpected":true,
                    "ownerSessionId":"default","valid":true,"staleReason":null
                },
                "expected_origin":origin,"expected_url":format!("{origin}/"),
                "consent_sha256":"a".repeat(64),"account_scope":"login.gov:user@example.test",
                "operation":operation
            }))
            .unwrap(),
        )
        .unwrap()
    }

    fn dom_fixture(expression: &str, setup: &str) -> Value {
        // Synthetic Node model verifies synchronous guard/dispatch semantics,
        // not Chromium DOM fidelity or installed-runtime observer isolation.
        let script = format!(
            r#"
          globalThis.top = globalThis;
          globalThis.location = {{origin:'https://secure.login.gov',href:'https://secure.login.gov/'}};
          let writes=0, clicks=0;
          class HTMLElement {{
            constructor() {{ this.isConnected=true; this.disabled=false; this.readOnly=false; }}
            getClientRects() {{ return [1]; }}
            hasAttribute() {{ return false; }}
            dispatchEvent() {{ if (this.onInput) this.onInput(); }}
            click() {{ clicks++; }}
          }}
          class HTMLInputElement extends HTMLElement {{
            get value() {{ return this._value || ''; }}
            set value(value) {{ writes++; this._value=value; }}
          }}
          class HTMLButtonElement extends HTMLElement {{
            get formAction() {{ return this._formAction || location.href; }}
          }}
          class HTMLFormElement extends HTMLElement {{}}
          globalThis.HTMLElement=HTMLElement; globalThis.HTMLInputElement=HTMLInputElement;
          globalThis.HTMLButtonElement=HTMLButtonElement; globalThis.HTMLFormElement=HTMLFormElement;
          globalThis.getComputedStyle=()=>({{visibility:'visible',display:'block'}});
          const form=new HTMLFormElement(); form.method='post'; form.action=location.href;
          const email=new HTMLInputElement(); email.type='email'; email.form=form;
          const password=new HTMLInputElement(); password.type='password'; password.form=form;
          const submit=new HTMLButtonElement(); submit.type='submit'; submit.form=form;
          const nodes={{'#email':[email],'#password':[password],'#submit':[submit]}};
          globalThis.document={{querySelectorAll: selector => nodes[selector] || []}};
          {setup}
          let result=null, failed=false;
          try {{ result={expression}; }} catch {{ failed=true; }}
          process.stdout.write(JSON.stringify({{writes,clicks,result,failed}}));
        "#
        );
        let mut child = Command::new("node")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("node is required for the synthetic private DOM tests");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(script.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        serde_json::from_slice(&output.stdout).unwrap()
    }

    #[test]
    fn synthetic_login_dispatches_once_without_returning_credentials() {
        let result = dom_fixture(&expression(&login()).unwrap(), "");
        assert_eq!(
            result,
            json!({"writes":2,"clicks":1,"result":{"dispatch_attempted":true},"failed":false})
        );
        assert!(!result.to_string().contains("SYNTHETIC_SECRET"));
    }

    #[test]
    fn synthetic_wrong_destination_and_ambiguous_controls_fail_before_secret_entry() {
        let expression = expression(&login()).unwrap();
        for setup in [
            "location.href='https://secure.login.gov/?different';",
            "location.origin='https://other.example';",
            "globalThis.top={};",
            "nodes['#password'].push(new HTMLInputElement());",
            "form.action='https://other.example/collect';",
            "form.method='get';",
            "submit.hasAttribute=name=>name==='formmethod'; submit.formMethod='get';",
            "submit.hasAttribute=name=>name==='formaction'; submit._formAction='https://other.example/collect';",
            "submit.hasAttribute=name=>name==='formtarget'; submit.formTarget='_blank';",
            "form.target='_blank';",
            "nodes['base[target]']=[{getAttribute:()=> '_blank'}];",
            "password.type='text';",
            "submit.type='button';",
            "password.form=new HTMLFormElement();",
        ] {
            assert_eq!(
                dom_fixture(&expression, setup),
                json!({"writes":0,"clicks":0,"result":null,"failed":true})
            );
        }
    }

    #[test]
    fn synthetic_input_event_navigation_prevents_password_and_submit() {
        let result = dom_fixture(
            &expression(&login()).unwrap(),
            "email.onInput=()=>{location.href='https://secure.login.gov/?replacement';};",
        );
        assert_eq!(
            result,
            json!({"writes":1,"clicks":0,"result":null,"failed":true})
        );
    }

    #[test]
    fn synthetic_backup_renew_and_key_read_require_matching_live_account() {
        for (origin, operation_data, setup, expected) in [
            ("https://secure.login.gov", json!({"kind":"backup_code","code_selector":"#password",
                "submit_selector":"#submit","account_selector":"#email","account_value":"user@example.test",
                "code":"SYNTHETICBACKUP1234","source_scope":"reviewed-message:slot-0"}),
                "",json!({"writes":1,"clicks":1,"result":{"dispatch_attempted":true},"failed":false})),
            ("https://sam.gov", json!({"kind":"renew","submit_selector":"#submit",
                "account_selector":"#email","account_value":"user@example.test"}),
                "location.origin='https://sam.gov'; location.href='https://sam.gov/'; form.action=location.href;",
                json!({"writes":0,"clicks":1,"result":{"dispatch_attempted":true},"failed":false})),
            ("https://sam.gov", json!({"kind":"read_key","selector":"#password",
                "account_selector":"#email","account_value":"user@example.test"}),
                "location.origin='https://sam.gov'; location.href='https://sam.gov/'; password._value='SYNTHETIC_PRIVATE_KEY_123456789';",
                json!({"writes":0,"clicks":0,"result":{"key":"SYNTHETIC_PRIVATE_KEY_123456789"},"failed":false})),
        ] {
            let expression = expression(&operation(origin,operation_data)).unwrap();
            assert_eq!(dom_fixture(&expression, &format!("{setup} email._value='user@example.test';")),expected);
            assert_eq!(dom_fixture(&expression, &format!("{setup} email._value='wrong@example.test';")),
                json!({"writes":0,"clicks":0,"result":null,"failed":true}));
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn private_transport_admits_once_encrypts_result_and_never_broadcasts_secret() {
        use super::super::private_identity::{validate_private_target, LivePrivateIdentity};
        use futures_util::{SinkExt, StreamExt};
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        use tokio::net::TcpListener;
        use tokio_tungstenite::tungstenite::Message;
        for (failed, read_key) in [(false, false), (true, false), (false, true)] {
            let page_url = if read_key {
                "https://sam.gov/"
            } else {
                "https://secure.login.gov/"
            };
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let endpoint = format!("ws://{}", listener.local_addr().unwrap());
            let count = Arc::new(AtomicUsize::new(0));
            let server_count = Arc::clone(&count);
            let server = tokio::spawn(async move {
                let (socket, _) = listener.accept().await.unwrap();
                let mut peer = tokio_tungstenite::accept_async(socket).await.unwrap();
                while let Some(Ok(message)) = peer.next().await {
                    if !message.is_text() {
                        break;
                    }
                    let command: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
                    let result = match command["method"].as_str().unwrap() {
                        "Target.getTargetInfo" => {
                            assert!(command["params"].get("targetId").is_none());
                            json!({"targetInfo":{"targetId":"retained","type":"page","url":page_url}})
                        }
                        "Page.getFrameTree" => {
                            json!({"frameTree":{"frame":{"id":"top","url":page_url}}})
                        }
                        "Page.createIsolatedWorld" => json!({"executionContextId":7}),
                        "Runtime.evaluate" => {
                            server_count.fetch_add(1, Ordering::SeqCst);
                            assert_eq!(command["params"]["contextId"], 7);
                            assert!(
                                read_key
                                    || command["params"]["expression"]
                                        .as_str()
                                        .unwrap()
                                        .contains("SYNTHETIC_SECRET")
                            );
                            // A private exception may contain page-originated
                            // secrets; it must not enter a public error/result.
                            if failed {
                                json!({"exceptionDetails":{"text":"SYNTHETIC_SECRET_PRIVATE_ERROR"}})
                            } else if read_key {
                                json!({"result":{"value":{"key":"SYNTHETIC_PRIVATE_KEY_123456789"}}})
                            } else {
                                json!({"result":{"value":{"dispatch_attempted":true}}})
                            }
                        }
                        _ => panic!("unexpected synthetic CDP command"),
                    };
                    if peer
                        .send(Message::Text(
                            json!({"id":command["id"],"result":result}).to_string(),
                        ))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            });
            let endpoint_fixture =
                crate::native::cdp::client::TestCdpEndpoint::new(&endpoint).unwrap();
            let client = endpoint_fixture.connect().await.unwrap();
            let mut raw = client.subscribe_raw();
            let operation = if read_key {
                operation(
                    "https://sam.gov",
                    json!({"kind":"read_key","selector":"#key",
                    "account_selector":"#email","account_value":"user@example.test"}),
                )
            } else {
                login()
            };
            let live = LivePrivateIdentity {
                profile_id: "sam",
                browser_id: "session:default",
                session_name: "default",
                target_id: "retained",
                url: operation.expected_url(),
                ready: true,
            };
            let target = validate_private_target(
                operation.service_tab_handle(),
                operation.service_tab_handle(),
                &live,
                operation.expected_origin(),
                operation.expected_url(),
            )
            .unwrap();
            let permit = client.begin_private_interval(&target).unwrap();
            let root =
                std::env::temp_dir().join(format!("ab-private-execution-{}", uuid::Uuid::new_v4()));
            let store = SecretStore::open(&root).unwrap();
            let reference = store
                .stage(b"synthetic authenticated operation fixture")
                .unwrap();
            let result = execute_staged_dom(
                &client,
                &permit,
                "page-session",
                &store,
                &reference,
                &operation,
            )
            .await;
            if failed {
                assert!(matches!(result, Err("private_execution_failed")));
                assert!(store.load_result(&reference).is_err());
            } else {
                if read_key {
                    assert!(matches!(result, Ok(PrivateStepOutcome::ResultStored)));
                } else {
                    assert!(matches!(result, Ok(PrivateStepOutcome::DispatchAttempted)));
                }
                let value: Value =
                    serde_json::from_slice(&store.load_result(&reference).unwrap()).unwrap();
                assert_eq!(
                    value,
                    if read_key {
                        json!({"key":"SYNTHETIC_PRIVATE_KEY_123456789"})
                    } else {
                        json!({"dispatch_attempted":true})
                    }
                );
                let ciphertext = std::fs::read(root.join(format!("{reference}.result"))).unwrap();
                assert!(!String::from_utf8_lossy(&ciphertext).contains("SYNTHETIC_PRIVATE_KEY"));
            }
            drop(store);
            let reopened = SecretStore::open(&root).unwrap();
            assert!(execute_staged_dom(
                &client,
                &permit,
                "page-session",
                &reopened,
                &reference,
                &operation
            )
            .await
            .is_err());
            assert_eq!(count.load(Ordering::SeqCst), 1);
            assert!(raw.try_recv().is_err());
            assert!(client
                .send_command_with_timeout(
                    "Runtime.evaluate",
                    None,
                    Some("page-session"),
                    Duration::from_millis(50)
                )
                .await
                .is_err());
            drop(permit);
            drop(client);
            server.abort();
            let _ = server.await;
            drop(reopened);
            std::fs::remove_dir_all(root).unwrap();
        }
    }
}
