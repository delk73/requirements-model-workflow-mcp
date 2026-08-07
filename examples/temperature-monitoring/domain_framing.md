# Temperature Monitoring Domain Framing

## Included Concerns

* Receiving temperature readings.
* Determining whether a temperature reading is accepted or rejected.
* Recording accepted temperature readings.
* Comparing an accepted temperature reading with a configured threshold.
* Producing an alert when an accepted temperature reading exceeds the configured threshold.

## Excluded Concerns

* How temperature is measured or how a reading is transmitted to the system.
* How readings are corrected, transformed, aggregated, or removed after receipt.
* The persistence technology, data format, or deployment environment used to record readings.
* The delivery channel, recipient, acknowledgement, or escalation process for alerts.
* Configuration management, threshold ownership, and threshold change procedures.
* Dashboards, reports, historical analysis, prediction, and automated control actions.

## Participants and External Dependencies

### Participants

* **Reading source:** Provides temperature readings to the system.
* **Temperature-monitoring system:** Receives, evaluates, records, and alerts on readings within the included scope.

### External Dependencies

* A source capable of providing temperature readings.
* A configured temperature threshold against which accepted readings can be compared.

The behavior and implementation of these external dependencies are outside this framing, except for the inputs they provide to the system.

## Competency Questions

1. What temperature readings has the system received?
2. For a received temperature reading, was it accepted or rejected?
3. Which temperature readings were accepted?
4. Which accepted temperature readings were recorded?
5. What configured threshold was used to evaluate an accepted reading?
6. Which accepted readings exceeded the configured threshold?
7. For which accepted readings did the system produce an alert?

## Unresolved Boundary Questions

* What makes a received temperature reading acceptable or rejectable?
* Is a reading evaluated only once, or may its acceptance decision be revisited?
* Does “exceeds” mean strictly greater than the threshold, excluding equality?
* Is the configured threshold a single value for the whole system, or can it vary by context?
* Must every accepted reading be recorded, and what does “recorded” mean at this boundary?
* What event constitutes producing an alert at the system boundary?
* What identity or context, if any, accompanies a temperature reading?
* How are duplicated, malformed, or otherwise unusual received readings treated?

