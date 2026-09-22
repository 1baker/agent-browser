# Review goal

Determine whether the agent-browser retained-browser and Cloudflare ingress repair is safe to ship.

The implementation must let a daemon session reuse a live retained browser when the remote-view route is intentionally owned by a different service session, without launching a duplicate browser or acquiring the already-open profile. It must preserve exact retained browser custody through publication, expose the ready Guacamole route through `agent-browser.bwkuehl.com`, and support a real authenticated ChatGPT browser round trip. Reject the work if the evidence is incomplete, the identity separation weakens route ownership checks, or the installed runtime differs from the reviewed source.
