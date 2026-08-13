---
rmwm:
  schema: "artifact/v1"
  id: "raw-adc-domain-framing"
  type: "domain_framing"
---

# Raw ADC Capture Domain Framing

## Included Concerns

* Receiving the analog output produced by the DRV425EVM.
* Sampling the analog output with an ADC.
* Producing a raw ADC record for each captured sample.
* Determining whether each raw ADC record is accepted or rejected.
* Retaining accepted raw ADC records.
* Retaining the acquisition context needed to interpret accepted raw ADC records.
* Applying acquisition context in capture-level and record-level layers.
* Keeping rejected raw ADC records distinguishable from accepted raw ADC records.

## Excluded Concerns

* The internal design of the DRV425EVM.
* Proving the physical accuracy or calibration of the sensor.
* Designing the ADC hardware or analog front end.
* Correcting, transforming, or aggregating captured values.
* Selecting a particular persistence technology or deployment environment.
* Defining user interfaces, dashboards, reports, or automated control actions.
* Selecting records for replay.
* Projecting records into canonical replay input.
* Validating or executing replay input.
* Producing replay execution records.
* Performing functional comparison or timing evaluation.
* Producing claim-bounded evaluations.
* Assembling portable replay evidence packages.

## Participants and External Dependencies

### Participants

* **DRV425EVM:** Produces the analog output presented to the ADC capture system.
* **ADC capture system:** Samples the analog output and produces, evaluates, and retains raw ADC records.

### External Dependencies

* A physical magnetic-field condition presented to the DRV425EVM.
* An acquisition configuration used during ADC capture.

The behavior and implementation of these external dependencies are outside this framing, except for the physical condition and acquisition configuration they provide to the included work.

## Competency Questions

1. Which raw ADC records were produced during a capture?
2. From which captured samples were the raw ADC records produced?
3. What acquisition context applies to each accepted raw ADC record?
4. For each raw ADC record, was it accepted or rejected?
5. Which accepted raw ADC records were retained?
6. How can rejected raw ADC records be distinguished from accepted raw ADC records?

## Unresolved Boundary Questions

* What event begins and ends a capture?
* What information must a raw ADC record contain?
* What gives a raw ADC record its identity?
* Must a raw ADC record preserve the captured ADC code without modification?
* What makes a raw ADC record acceptable or rejectable?
* Is each raw ADC record evaluated only once?
* What information about a rejected raw ADC record must be retained?
* What acquisition context is required to interpret an accepted raw ADC record?
* Which acquisition context applies to the entire capture, and which applies to an individual raw ADC record?
* How are capture-level and record-level acquisition context combined when interpreting a raw ADC record?
* May one record-level acquisition context apply to a group of raw ADC records?
* What does retaining a raw ADC record mean at this boundary?
* Must capture order or timing be retained?
* How are duplicated, malformed, missing, or out-of-order samples or records handled?
