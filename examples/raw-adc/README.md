# Raw ADC Capture Example

This example records a manual application of the story-to-ontology portion of
the Requirements Model Workflow using the DRV425EVM raw-ADC capture path. The
example is domain-specific. The workflow remains domain-neutral.

## Current Artifacts

* [System story](story.md) provides the accepted source narrative.
* [Domain framing](domain_framing.md) provides the accepted scope, participants,
  external dependencies, competency questions, and unresolved boundaries.
* [Domain ontology](domain_ontology.md) provides the accepted concepts,
  properties, relationships, constraints, probe propositions, and support paths.
* [Requirements-model manifest](requirements_model.yaml) records the accepted
  revisions and source bindings.

## Current State

The system story, domain framing, and domain ontology are accepted and bound in
the requirements-model manifest.

This example currently establishes the manual story-to-ontology baseline. It
does not yet include controlled vocabulary, requirements, requirement
decomposition, or traceability artifacts.

The MCP runtime has not been implemented. This example is a reference fixture
for implementing and testing the gated workflow. It does not demonstrate
automated lifecycle enforcement.
