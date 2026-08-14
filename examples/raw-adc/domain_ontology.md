---
rmwm:
  schema: "artifact/v1"
  id: "raw-adc-domain-ontology"
  type: "domain_ontology"
---

# Raw ADC Capture Domain Ontology

## Ontology Probe 1: Capture Records

### Competency Question

Which raw ADC records were produced during a capture?

### Concepts

| Concept | Relational equivalent | Meaning |
| --- | --- | --- |
| Capture | Record type | Candidate grouping for raw ADC sampling |
| Raw ADC Record | Record type | The raw record produced from a captured ADC sample |
| Acquisition Context | Record type | Information needed to interpret acquired raw ADC data |
| Captured Sample | Record type | One ADC sample that results in a raw ADC record |
| DRV425EVM | Record type | The sensing device that produces the analog output presented for capture |
| Analog Output | Record type | The DRV425EVM output received during a capture |

### Properties

| Concept | Property | Purpose |
| --- | --- | --- |
| Capture | Capture identity | Distinguishes one capture from another |
| Raw ADC Record | Record identity | Distinguishes records produced during a capture |
| Raw ADC Record | Raw ADC value | Preserves the captured ADC code |
| Raw ADC Record | Acceptance status | States whether the record was accepted or rejected |
| Acquisition Context | Context identity | Distinguishes one acquisition context from another |
| Acquisition Context | Sampling timing definition | Defines timing used to interpret acquired raw ADC data |
| Captured Sample | Sample identity | Distinguishes one captured sample from another |
| Analog Output | Analog output identity | Distinguishes one analog output from another |

The representation of each property remains unresolved.

### Relationships

| Source | Relationship | Target |
| --- | --- | --- |
| Capture | produces | Raw ADC Record |
| Capture | uses | Acquisition Context |
| Raw ADC Record | results from | Captured Sample |
| Raw ADC Record | has | Acquisition Context |
| DRV425EVM | produces | Analog Output |
| Capture | receives | Analog Output |

### Constraints

* Each captured sample results in exactly one raw ADC record.
* Each raw ADC record results from exactly one captured sample.
* Each raw ADC record has an acceptance status.
* Acceptance status is either `accepted` or `rejected`.
* Each accepted raw ADC record is retained.
* An acquisition context applies either to a capture or to one or more raw ADC records.
* Each accepted raw ADC record is interpreted using the acquisition context of its capture.
* An accepted raw ADC record may also have record-level acquisition context.
* The effective acquisition context for an accepted raw ADC record combines its capture-level context with any record-level context.
* Each analog output received during a capture is produced by a DRV425EVM.

### Probe Propositions

* The probe represents a capture as a candidate grouping for raw ADC records.
  The event that begins and ends a capture remains unresolved.
* The probe proposes that each raw ADC record belongs to the capture that
  produced it and that a capture may produce multiple raw ADC records.
* The probe represents acquisition context in capture-level and record-level
  layers. The information assigned to each layer and the rules for combining
  the layers remain unresolved.
* The probe represents the accepted-or-rejected result as the acceptance status
  of a raw ADC record. Whether the underlying evaluation may be repeated or
  revisited remains unresolved.
* The probe does not specify whether sampling is uniform.
* The probe represents receipt as a relationship between a capture and an analog output.
  What establishes analog-output identity remains unresolved.
* The boundaries or duration of an analog output remain unresolved.
* The number of analog outputs associated with a capture remains unresolved.

### Support Paths

| Ontology probe element | Support path |
| --- | --- |
| Capture | Competency question: “Which raw ADC records were produced during a capture?” |
| Raw ADC Record | Story: “An ADC capture system … produces raw ADC records.” |
| Raw ADC Record — Acceptance status | Story: for each captured sample, the system determines whether to accept or reject the resulting raw ADC record. |
| Acquisition Context | Story: accepted records are retained with the acquisition context needed to interpret them. |
| Capture produces Raw ADC Record | Story: the system produces raw ADC records from captured samples. |
| Capture uses Acquisition Context | Domain framing: acquisition context uses capture-level and record-level layers. |
| Acquisition Context — Context identity | Competency question 3 requires applicable acquisition context to be distinguishable. |
| Raw ADC Record has Acquisition Context | Domain framing: acquisition context includes a record-level layer. |
| Acquisition Context applies to Capture or Raw ADC Record | Domain framing: acquisition context uses capture-level and record-level layers. |
| Capture-level Acquisition Context applies to accepted Raw ADC Record | Story: accepted raw ADC records are retained with the acquisition context needed to interpret them. |
| Accepted Raw ADC Record may have record-level Acquisition Context | Domain framing: acquisition context includes a record-level layer. |
| Effective Acquisition Context combines capture and record layers | Domain framing: acquisition context is applied in capture-level and record-level layers. |
| Each Raw ADC Record has Acceptance status | Story: the system makes an accept-or-reject determination for each resulting raw ADC record. |
| Acceptance status is accepted or rejected | Story: the permitted outcomes are accept and reject. |
| Each accepted Raw ADC Record is retained | Story: accepted raw ADC records are retained together with the acquisition context needed to interpret them. |
| Captured Sample | Story: “For each captured sample … the resulting raw ADC record.” |
| Captured Sample — Sample identity | Competency question 2 requires the source sample of each raw ADC record to be distinguishable. |
| Raw ADC Record results from Captured Sample | Story: each captured sample has a resulting raw ADC record. |
| One captured sample results in exactly one Raw ADC Record | Story: “For each captured sample … the resulting raw ADC record.” |
| Each Raw ADC Record results from exactly one Captured Sample | Story: the system produces raw ADC records from captured samples and refers to the resulting raw ADC record for each captured sample. |
| DRV425EVM | Story: “A DRV425EVM produces an analog output …” |
| Analog Output | Story: “A DRV425EVM produces an analog output … An ADC capture system samples that output …” |
| Analog Output — Analog output identity | Competency question 7 requires the received analog output to be distinguishable. |
| DRV425EVM produces Analog Output | Story: “A DRV425EVM produces an analog output …” |
| Capture receives Analog Output | Domain framing included concern: “Receiving the analog output produced by the DRV425EVM.” Competency question 7: “Which DRV425EVM analog output was received during a capture?” |
| Each Analog Output received during a Capture is produced by a DRV425EVM | Story: the DRV425EVM produces an analog output. Domain framing: the analog output produced by the DRV425EVM is received. |
