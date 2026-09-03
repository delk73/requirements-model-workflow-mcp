# Requirements Model Workflow MCP

An MCP workflow for iteratively developing a system story into a reviewed domain ontology, controlled vocabulary, normative requirements, requirement decomposition, and deterministic traceability.

> **License status:** Temporarily proprietary. Reuse and automated ingestion
> are not authorized. See [LICENSE](LICENSE).

Provenance: `RMWM-PROVENANCE-2026-08-07-01`

Status: Initial draft

## Project Definition

* [Product contract](docs/product_contract.md)
* [Raw-ADC capture example](examples/raw-adc/)

## Workflow

story provides the source narrative
→ domain framing selects the domain scope
→ competency questions define what the model must answer
→ an ontology probe models that scope and tests those questions
→ findings revise the story, framing, questions, or ontology
→ the loop repeats until the accepted model has sufficient support
→ controlled vocabulary names accepted ontology elements consistently
→ requirements constrain accepted ontology elements normatively
→ decomposition refines accepted requirements
→ traceability connects accepted sources, model elements, and requirements

Human review and revision apply throughout the workflow.

A stage may produce an accepted revision that permits work on a later stage. Acceptance does not close the stage. Findings from later stages may require a new revision of an earlier stage and review of dependent work.

Workflow-managed artifacts use controlled frontmatter for stable artifact
identity and type. The requirements-model manifest identifies accepted
revisions and source bindings. Staged-candidate state remains owned by the
MCP.

## Raw ADC MCP reference run

A developer-facing grounding run that exercises the real MCP STDIO
JSON-RPC surface end to end: it launches the built server as a child
process, drives it through `initialize`, `tools/list`, and the
candidate/review/acceptance lifecycle for the Raw ADC example, then
reaccepts an upstream `requirements` artifact and confirms an accepted
`requirement_decomposition` artifact transitions to `review_required`.

Run it with:

```sh
cargo run --bin raw_adc_reference_run
```

Expected result:

```
RESULT: PASS
```

The run operates on an isolated temporary copy of
[examples/raw-adc](examples/raw-adc/) and never modifies the checked-in
example.

