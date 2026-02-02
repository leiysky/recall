# ADR-2026-02-02-rql-pipeline-clause-order

Status: accepted
Context:
- Users asked for a pipeline-friendly RQL clause order with FROM first and SELECT last.
- Interface rules require backward-compatible RQL syntax changes.
Decision:
- Accept FROM-first / SELECT-last as the canonical RQL order.
- Keep SELECT-first syntax supported for compatibility.
- Update docs/examples to present pipeline style and note legacy support.
Consequences:
- New pipeline style is documented and covered by tests.
- Legacy queries continue to parse without changes.
Alternatives:
- Break compatibility by enforcing a single new order (rejected).
- Keep SELECT-first as the only order (rejected).
Links:
- ISSUE-2026-02-02-rql-pipeline-clause-order
