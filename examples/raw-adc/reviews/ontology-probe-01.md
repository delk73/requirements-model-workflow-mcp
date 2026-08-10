# Raw ADC Ontology Probe 1 Review

## Test Subject

- Workflow stage: `domain_ontology`
- Artifact: `raw-adc-domain-ontology`
- Tested content: `sha256:75f11a1176de1b576146d290a64b11cd8cd10eb79bcadeb59db055c262626748`
- Artifact state: draft

## Competency Question

Which raw ADC records were produced during a capture?

## Test Result

Pass.

Given a capture identity, the probe follows the `produces` relationship from
`Capture` to `Raw ADC Record` and returns the identities of the related records.

## Findings

1. The probe can answer which raw ADC records were produced during a selected
   capture.
2. The answer requires capture identity, raw ADC record identity, and a
   connection from the capture to its produced records.
3. Capture boundaries and identity representations remain unresolved. They do
   not prevent the structural answer.
4. Acquisition context and sampling timing are not required to answer this
   competency question. This test does not validate them.
5. The test does not determine whether every captured sample produces exactly
   one raw ADC record. Competency question 2 addresses that issue.

## Finding Routes

| Finding | Route | Required action |
| --- | --- | --- |
| The probe answers competency question 1 | Review record | Retain the passing result |
| Capture identity, record identity, and their connection are required | Domain ontology | Already represented; no model change |
| Capture boundaries and identity representations remain unresolved | Domain framing | Already recorded; no framing change |
| Acquisition context and timing were not tested | Review record | Retain the limit of this test |
| Sample-to-record cardinality was not tested | Competency question 2 | Test in the next ontology-probe cycle |
| Findings require durable storage | Product contract | Define the generic review-record artifact |

## Required Model Revisions

None.

## Retest

Not required. This review did not identify a required change to the story,
domain framing, competency question, or ontology probe.

## Review Decision

The ontology probe is sufficient to answer competency question 1. This decision
does not accept the domain ontology.
