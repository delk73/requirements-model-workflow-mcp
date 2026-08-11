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
| Acquisition Context | Record type | The shared information needed to interpret a capture |
| Captured Sample | Record type | One ADC sample that results in a raw ADC record |

### Properties

| Concept | Property | Purpose |
| --- | --- | --- |
| Capture | Capture identity | Distinguishes one capture from another |
| Raw ADC Record | Record identity | Distinguishes records produced during a capture |
| Raw ADC Record | Raw ADC value | Preserves the captured ADC code |
| Acquisition Context | Sampling timing definition | Defines the timing used to interpret the capture |
| Captured Sample | Sample identity | Distinguishes one captured sample from another |

The representation of each property remains unresolved.

### Relationships

| Source | Relationship | Target |
| --- | --- | --- |
| Capture | produces | Raw ADC Record |
| Capture | uses | Acquisition Context |
| Raw ADC Record | results from | Captured Sample |

### Constraints

* Each captured sample results in exactly one raw ADC record.
* Each raw ADC record results from exactly one captured sample.

### Probe Propositions

* The probe represents a capture as a candidate grouping for raw ADC records.
  The event that begins and ends a capture remains unresolved.
* The probe proposes that each raw ADC record belongs to the capture that
  produced it and that a capture may produce multiple raw ADC records.
* The probe provisionally associates acquisition context with a capture. The
  required scope of that context remains unresolved.
* The probe does not specify whether sampling is uniform.

### Support Paths

| Ontology probe element | Support path |
| --- | --- |
| Capture | Competency question: “Which raw ADC records were produced during a capture?” |
| Raw ADC Record | Story: “An ADC capture system … produces raw ADC records.” |
| Acquisition Context | Story: accepted records are retained with the acquisition context needed to interpret them. |
| Capture produces Raw ADC Record | Story: the system produces raw ADC records from captured samples. |
| Capture uses Acquisition Context | Probe proposition: acquisition context is associated with a capture for testing; the story directly supports retaining accepted records with acquisition context. |
| Captured Sample | Story: “For each captured sample … the resulting raw ADC record.” |
| Captured Sample — Sample identity | Competency question 2 requires the source sample of each raw ADC record to be distinguishable. |
| Raw ADC Record results from Captured Sample | Story: each captured sample has a resulting raw ADC record. |
| One captured sample results in exactly one Raw ADC Record | Story: “For each captured sample … the resulting raw ADC record.” |
| Each Raw ADC Record results from exactly one Captured Sample | Story: the system produces raw ADC records from captured samples and refers to the resulting raw ADC record for each captured sample. |
