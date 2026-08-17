---
rmwm:
  schema: "artifact/v1"
  id: "raw-adc-domain-ontology"
  type: "domain_ontology"
---

# Raw ADC Capture Domain Ontology

## Competency Questions

The ontology addresses all competency questions in the domain framing.

## Concepts

| Concept | Relational equivalent | Meaning |
| --- | --- | --- |
| Capture | Record type | Candidate grouping for raw ADC sampling |
| Raw ADC Record | Record type | The raw record produced from a captured ADC sample |
| Acquisition Context | Record type | Information needed to interpret acquired raw ADC data |
| Capture Timing Basis | Record type | Timing information used to interpret samples from a capture |
| Captured Sample | Record type | One ADC sample that results in a raw ADC record |
| Sample Timing Observation | Record type | A retained timing observation for one captured sample |
| DRV425EVM | Record type | The sensing device that produces the analog output presented for capture |
| Analog Output | Record type | The DRV425EVM output received during a capture |

## Properties

| Concept | Property | Purpose |
| --- | --- | --- |
| Capture | Capture identity | Distinguishes one capture from another |
| Raw ADC Record | Record identity | Distinguishes records produced during a capture |
| Raw ADC Record | Acceptance status | States whether the record was accepted or rejected |
| Acquisition Context | Context identity | Distinguishes one acquisition context from another |
| Capture Timing Basis | Timing basis identity | Distinguishes one capture timing basis from another |
| Captured Sample | Sample identity | Distinguishes one captured sample from another |
| Captured Sample | Capture order position | Establishes the sample’s relative position within its capture |
| Sample Timing Observation | Timing observation identity | Distinguishes one sample timing observation from another |
| Analog Output | Analog output identity | Distinguishes one analog output from another |

The representation of each property remains unresolved.

## Relationships

| Source | Relationship | Target |
| --- | --- | --- |
| Capture | produces | Raw ADC Record |
| Capture | uses | Acquisition Context |
| Acquisition Context | identifies | Capture Timing Basis |
| Raw ADC Record | results from | Captured Sample |
| Captured Sample | has | Sample Timing Observation |
| Raw ADC Record | has | Acquisition Context |
| DRV425EVM | produces | Analog Output |
| Capture | receives | Analog Output |
| Analog Output | is sampled to produce | Captured Sample |

## Constraints

* Each captured sample results in exactly one raw ADC record.
* Each raw ADC record results from exactly one captured sample.
* Each raw ADC record is produced during exactly one capture.
* Each raw ADC record has exactly one acceptance status, either `accepted` or
  `rejected`.
* Each captured sample has exactly one capture order position within its capture.
* No two captured samples in the same capture have the same capture order position.
* Capture order positions place all samples within a capture in one unambiguous order.
* The Capture order position of each Captured Sample is retained.
* Each Captured Sample has zero or more Sample Timing Observations.
* Each Sample Timing Observation belongs to exactly one Captured Sample.
* Each accepted raw ADC record is retained.
* Each Acquisition Context used by a Capture is capture-level context for the Raw
  ADC Records produced during that Capture.
* Every Capture has at least one Capture Timing Basis identified by an Acquisition
  Context it uses.
* Each Capture’s timing bases are retained.
* A Capture’s timing bases apply to every Captured Sample in that Capture.
* Each Acquisition Context that a Raw ADC Record has is record-level context for
  that Raw ADC Record.
* Each Acquisition Context applicable to an accepted Raw ADC Record is retained.
* Each analog output received during a capture is produced by a DRV425EVM.

## Probe Propositions

* The probe represents a capture as a candidate grouping for raw ADC records.
  The event that begins and ends a capture remains unresolved.
* The probe proposes that each raw ADC record belongs to the capture that
  produced it and that a capture may produce multiple raw ADC records.
* The probe represents acquisition context in capture-level and record-level
  layers. Which information belongs at each layer, how the layers are combined,
  how conflicts between layers are resolved, and whether one record-level
  context may apply to multiple records remain unresolved.
* The probe uses capture order position to represent the relative order of
  captured samples within a capture. How an order position is represented,
  whether positions must be contiguous, and how missing samples or gaps appear
  in the order remain unresolved.
* The probe requires Capture order positions and Capture Timing Bases to be
  retained. It does not specify how they are stored.
* The probe represents the accepted-or-rejected result as the acceptance status
  of a raw ADC record. Whether the underlying evaluation may be repeated or
  revisited remains unresolved.
* The probe does not specify whether sampling is uniform.
* The probe links a Capture to its timing bases through Acquisition Context.
  The contents and representation of a timing basis remain unresolved.
  Configured timing describes intended sampling timing. It does not show when
  samples actually occurred.
* Sample Timing Observations are separate from Capture Timing Bases and Capture
  order positions.
* The event, time reference, units, resolution, precision, uncertainty,
  contents, and representation of a Sample Timing Observation remain unresolved.
* If a Captured Sample has no Sample Timing Observation, the model makes no claim
  about timing measurements outside the retained data.
* The probe represents receipt as a relationship between a capture and an analog output.
  What establishes analog-output identity remains unresolved.
* The boundaries or duration of an analog output remain unresolved.
* The number of analog outputs associated with a capture remains unresolved.

## Support Paths

| Ontology probe element | Support path |
| --- | --- |
| Capture | Competency question: “Which raw ADC records were produced during a capture?” |
| Raw ADC Record | Story: “An ADC capture system … produces raw ADC records.” Domain framing included concern: “Producing a raw ADC record for each captured sample.” |
| Raw ADC Record — Acceptance status | Story: for each captured sample, the system determines whether to accept or reject the resulting raw ADC record. Domain framing included concern: “Determining whether each raw ADC record is accepted or rejected.” |
| Acquisition Context | Story: accepted records are retained with the acquisition context needed to interpret them. Domain framing included concern: “Retaining the acquisition context needed to interpret accepted raw ADC records.” |
| Capture produces Raw ADC Record | Competency question 1: “Which raw ADC records were produced during a capture?” Probe proposition: each raw ADC record belongs to the capture that produced it and a capture may produce multiple raw ADC records. |
| Capture uses Acquisition Context | Domain framing: acquisition context uses capture-level and record-level layers. Probe proposition: the information assigned to each layer remains unresolved. |
| Capture Timing Basis | Domain framing included concern: “Retaining the capture-level timing configuration or timing basis needed to interpret captured samples.” |
| Acquisition Context identifies Capture Timing Basis | Domain framing included concern: “Retaining the capture-level timing configuration or timing basis needed to interpret captured samples.” Competency question 10: “What capture-level timing configuration or timing basis applies to the captured samples?” |
| Capture Timing Basis — Timing basis identity | Competency question 10 requires the applicable capture timing basis to be distinguishable. |
| Acquisition Context — Context identity | Competency question 3 requires applicable acquisition context to be distinguishable. |
| Raw ADC Record has Acquisition Context | Domain framing: acquisition context uses capture-level and record-level layers. Probe proposition: whether one record-level context may apply to multiple records remains unresolved. |
| Each Raw ADC Record has exactly one Acceptance status, either accepted or rejected | Direct story support: for each captured sample, the system determines whether to accept or reject the resulting raw ADC record. Domain framing included concern: “Determining whether each raw ADC record is accepted or rejected.” Competency question 4: “For each raw ADC record, was it accepted or rejected?” |
| Each accepted Raw ADC Record is retained | Story: accepted raw ADC records are retained together with the acquisition context needed to interpret them. Domain framing included concern: “Retaining accepted raw ADC records.” Competency question 5: “Which accepted raw ADC records were retained?” |
| Every Capture has at least one Capture Timing Basis identified by an Acquisition Context it uses | Domain framing included concern: “Retaining the capture-level timing configuration or timing basis needed to interpret captured samples.” Competency question 10: “What capture-level timing configuration or timing basis applies to the captured samples?” |
| Each Capture’s timing bases are retained | Domain framing included concern: “Retaining the capture-level timing configuration or timing basis needed to interpret captured samples.” Competency question 12: “For each captured sample, which timing information is retained: its capture timing basis and order position only, or those plus one or more sample timing observations?” |
| A Capture’s timing bases apply to every Captured Sample in that Capture | Domain framing included concern: “Retaining the capture-level timing configuration or timing basis needed to interpret captured samples.” Competency question 10: “What capture-level timing configuration or timing basis applies to the captured samples?” |
| Each Acquisition Context used by a Capture is capture-level context for the Raw ADC Records produced during that Capture | Domain framing support: acquisition context is applied in capture-level and record-level layers. Competency-question support: question 3 asks what retained acquisition context applies to each accepted raw ADC record. Probe proposition: which information belongs at each layer remains unresolved. |
| Each Acquisition Context that a Raw ADC Record has is record-level context for that Raw ADC Record | Domain framing support: acquisition context is applied in capture-level and record-level layers. Competency-question support: question 3 asks what retained acquisition context applies to each accepted raw ADC record. Probe proposition: whether one record-level context may apply to multiple records remains unresolved. |
| Each Acquisition Context applicable to an accepted Raw ADC Record is retained | Direct story support: accepted raw ADC records are retained together with the acquisition context needed to interpret them. Domain framing support: retaining the acquisition context needed to interpret accepted raw ADC records. Competency-question support: question 3 asks what retained acquisition context applies to each accepted raw ADC record. Probe proposition: how layers are combined and conflicts between layers are resolved remain unresolved. |
| Captured Sample | Story: “For each captured sample … the resulting raw ADC record.” Domain framing included concern: “Sampling the analog output with an ADC.” Domain framing included concern: “Producing a raw ADC record for each captured sample.” |
| Captured Sample — Sample identity | Competency question 2 requires the source sample of each raw ADC record to be distinguishable. |
| Captured Sample — Capture order position | Domain framing included concern: “Retaining the order of captured samples.” Competency question 9: “In what order were captured samples produced during a capture?” |
| The Capture order position of each Captured Sample is retained | Domain framing included concern: “Retaining the order of captured samples.” Competency question 12: “For each captured sample, which timing information is retained: its capture timing basis and order position only, or those plus one or more sample timing observations?” |
| Sample Timing Observation | Domain framing included concern: “Retaining sample-level timing observations separately from capture-level configured timing when such observations are available.” Competency question 11: “What sample-level timing observations, if any, apply to each captured sample?” |
| Sample Timing Observation — Timing observation identity | Domain framing included concern: “Retaining sample-level timing observations separately from capture-level configured timing when such observations are available.” Competency question 11: “What sample-level timing observations, if any, apply to each captured sample?” |
| Captured Sample has Sample Timing Observation | Domain framing included concern: “Retaining sample-level timing observations separately from capture-level configured timing when such observations are available.” Competency question 11: “What sample-level timing observations, if any, apply to each captured sample?” |
| Each Captured Sample has zero or more Sample Timing Observations | Domain framing included concern: “Retaining sample-level timing observations separately from capture-level configured timing when such observations are available.” Competency question 11: “What sample-level timing observations, if any, apply to each captured sample?” |
| Each Sample Timing Observation belongs to exactly one Captured Sample | Domain framing included concern: “Retaining sample-level timing observations separately from capture-level configured timing when such observations are available.” Competency question 11: “What sample-level timing observations, if any, apply to each captured sample?” |
| Raw ADC Record results from Captured Sample | Story: each captured sample has a resulting raw ADC record. Domain framing included concern: “Producing a raw ADC record for each captured sample.” Competency question 2: “From which captured samples were the raw ADC records produced?” |
| One captured sample results in exactly one Raw ADC Record | Story: “For each captured sample … the resulting raw ADC record.” Domain framing included concern: “Producing a raw ADC record for each captured sample.” |
| Each Raw ADC Record results from exactly one Captured Sample | Story: the system produces raw ADC records from captured samples and refers to the resulting raw ADC record for each captured sample. Competency question 2: “From which captured samples were the raw ADC records produced?” |
| Each captured sample has exactly one capture order position within its capture | Domain framing included concern: “Retaining the order of captured samples.” Competency question 9: “In what order were captured samples produced during a capture?” |
| No two captured samples in the same capture have the same capture order position | Domain framing included concern: “Retaining the order of captured samples.” Competency question 9: “In what order were captured samples produced during a capture?” |
| Capture order positions place all samples within a capture in one unambiguous order | Domain framing included concern: “Retaining the order of captured samples.” Competency question 9: “In what order were captured samples produced during a capture?” |
| Each Raw ADC Record is produced during exactly one Capture | Competency question 1 requires records to be identified during a capture. Probe proposition: each raw ADC record belongs to the capture that produced it. |
| DRV425EVM | Story: “A DRV425EVM produces an analog output …” Domain framing participant: “Produces the analog output presented to the ADC capture system.” |
| Analog Output | Story: “A DRV425EVM produces an analog output … An ADC capture system samples that output …” Domain framing included concern: “Receiving the analog output produced by the DRV425EVM.” |
| Analog Output — Analog output identity | Competency question 7 requires the received analog output to be distinguishable. |
| DRV425EVM produces Analog Output | Story: “A DRV425EVM produces an analog output …” Domain framing participant: “Produces the analog output presented to the ADC capture system.” Domain framing included concern: “Receiving the analog output produced by the DRV425EVM.” |
| Capture receives Analog Output | Domain framing included concern: “Receiving the analog output produced by the DRV425EVM.” Competency question 7: “Which DRV425EVM analog output was received during a capture?” |
| Each Analog Output received during a Capture is produced by a DRV425EVM | Story: the DRV425EVM produces an analog output. Domain framing: the analog output produced by the DRV425EVM is received. |
| Analog Output is sampled to produce Captured Sample | Story: “An ADC capture system samples that output …” Domain framing included concern: “Sampling the analog output with an ADC.” Competency question 8: “Which captured samples were produced by sampling that analog output?” |
