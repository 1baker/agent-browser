# Durable Task Outcome Receipts

P107 made admission crash-safe: the broker cursor advances and a command-bound
receipt is written before dispatch. That is the correct replay boundary, but an
admission receipt alone cannot say whether the browser command completed,
failed normally, or died before a response became durable.

P108 keeps the cursor semantics unchanged and adds a second durable boundary
before response publication. The daemon records `completed` or `failed`, a
SHA-256 digest and byte count of the exact response envelope, optional
post-action target and URL evidence, and a finalization timestamp. Repeating
the same finalization is idempotent; different terminal evidence is rejected.

An admitted receipt with no terminal record is projected as `indeterminate`.
It remains consumed and cannot be replayed. Recovery therefore requires an
operator to inspect status, revoke the old authority, and issue a newly
confirmed plan rather than guessing whether the stranded action ran.
