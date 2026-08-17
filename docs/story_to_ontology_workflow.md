# Story-to-Ontology Workflow

## Purpose

This workflow governs the path from an accepted system story to:

* an accepted domain framing
* an accepted domain ontology bound to that framing.

It separates deterministic artifact lifecycle work, agent semantic work, human
decisions, and orchestration. It does not define domain meaning, final MCP API
names, or Raw ADC-specific behavior.

The authority order is:

1. `docs/product_contract.md` defines normative product behavior.
2. Accepted artifacts and their manifest bindings define accepted model state.
3. `docs/story_to_ontology_workflow.md` defines how the story-to-ontology workflow applies the product contract.
4. MCP interfaces, orchestration prompts, and tests must conform to those documents.

The supporting Raw ADC evidence is the accepted model state at commit `b3b2d0bfda1b37bbe0c8915e51218595f11991ea`.

If this workflow document conflicts with `docs/product_contract.md`, the
product contract governs. The accepted Raw ADC model and Git history are
supporting evidence only for the workflow rules stated here. The audit
transcript is not an authority and must not be retained as a product artifact.

## Responsibility Boundary

### MCP

The MCP owns:

* accepted artifact reads
* candidate staging
* exact prospective bytes
* controlled frontmatter
* encoding and line endings
* content digests and byte counts
* revision handles
* source and target bindings
* stale-candidate detection
* lifecycle state
* deterministic structural validation
* acceptance of an explicitly approved candidate
* manifest updates
* transaction safety and recovery.

### Agent

The agent owns:

* proposing framing and ontology content
* reviewing semantic coverage
* testing competency questions against the model
* identifying findings
* proposing a route for each finding
* preparing revised candidate bodies
* comparing replay results with an accepted baseline.

The agent does not approve or accept its own semantic work.

### Human

The human owns:

* approval or rejection of framing meaning
* approval or rejection of ontology meaning
* approval of changes that resolve an unresolved boundary
* approval of one exact staged candidate for acceptance.

### Orchestration

The orchestration layer owns sequencing operations, presenting one bounded task
at a time, stopping at gates, preventing later steps from running early,
carrying candidate and review identifiers between steps, and requesting
explicit human decisions.

Orchestration does not create artifact identity or decide domain meaning.

## Public Operations and Internal Mechanics

The workflow exposes the following conceptual public capabilities:

* inspect workflow state
* read an accepted artifact
* stage an exact candidate
* record a semantic review
* record finding routes
* record candidate approval
* record candidate rejection
* withdraw a candidate
* replace a candidate
* accept an approved candidate
* report affected downstream artifacts.

These dispositions remain distinct:

* rejection records a negative decision
* withdrawal removes a candidate from further consideration
* replacement creates a new candidate that identifies the candidate it supersedes
* approval identifies one exact candidate as eligible for acceptance
* acceptance changes accepted model state.

The final operation names, request schemas, response schemas, transport
details, and storage layout are intentionally not defined here.

The following are internal mechanics, not separate semantic workflow steps:

* constructing controlled artifact bytes
* validating frontmatter
* applying encoding and line-ending rules
* calculating content digests
* calculating byte counts
* calculating revision handles
* checking accepted source revisions and the expected target state
* detecting stale candidates
* deriving lifecycle state
* validating controlled references
* committing artifact and manifest changes atomically
* recovering from incomplete writes.

## Candidate Lifecycle

The primary lifecycle is:

```text
prepared
-> staged
-> under_review
-> approved
-> accepted
```

Alternate outcomes are:

```text
staged -> rejected
staged -> withdrawn
staged -> replaced
under_review -> rejected
under_review -> replaced
staged -> stale
under_review -> stale
approved -> stale
```

Semantic testing applies to an exact staged candidate. A candidate records its
exact prospective bytes, its expected current target state, and the accepted
source revisions it uses. The expected target state is either a specific
accepted revision or the explicit absence of an accepted revision.

A revised body must be staged as a replacement candidate before retesting.
Retesting applies to the replacement candidate, not the superseded candidate.
Approval applies to one exact staged candidate. Acceptance must commit the same bytes that were approved and must revalidate the candidate’s expected target state and accepted source revisions.

A change to the expected target state or any accepted source revision makes the
candidate stale, and a stale candidate
cannot be accepted. Candidate replacement does not silently transfer approval
from the prior candidate.

## Story-to-Framing Flow

The framing flow is:

```text
inspect accepted story and model state
-> agent prepares framing body
-> MCP stages exact framing candidate against the accepted story
-> MCP performs deterministic validation
-> agent reviews the exact staged framing candidate
-> MCP records findings and proposed routes
-> human reviews framing meaning
-> agent prepares a revision if required
-> MCP stages the revision as a replacement candidate
-> MCP performs deterministic validation on the replacement
-> agent retests the exact replacement
-> human approves one exact staged framing candidate
-> MCP revalidates the candidate
-> MCP accepts the same exact bytes against the accepted story revision
```

The framing review must cover included concerns, excluded concerns,
participants, external dependencies, competency questions, and unresolved
boundary questions.

## Question-by-Question Ontology Loop

For each competency question, the loop is:

```text
select one accepted competency question
-> agent prepares an ontology candidate or replacement body
-> MCP stages exact prospective bytes against the accepted framing revision
-> MCP performs deterministic validation
-> agent tests the selected question against that exact staged candidate
-> agent records the structured query path
-> agent records where the path succeeds or breaks
-> agent records findings
-> agent proposes one route for each finding
-> human reviews meaning when required
-> agent prepares a revised body if required
-> MCP stages it as a replacement candidate
-> MCP performs deterministic validation on the replacement
-> agent retests the exact replacement
-> mark the question covered, deferred, or excluded
```

Every question must finish with exactly one of these states:

* covered
* deferred with an explicit unresolved boundary or
* excluded by the accepted framing.

An unclassified question must not disappear from the workflow.

## Framing Feedback Loop

When ontology review identifies a framing defect, use this route:

```text
ontology review identifies a framing defect
-> route the finding to domain framing
-> stop ontology acceptance work
-> run the Story-to-Framing Flow using the current accepted framing as the
   expected target state and the accepted story revision as its source
-> mark ontology candidates bound to the prior framing revision stale
-> mark the accepted ontology review_required when applicable
-> prepare a new ontology candidate against the new accepted framing
-> repeat affected competency-question tests
-> repeat the integrated ontology audit
```

Ontology work must not silently resolve a framing boundary.

## Deterministic Validation Boundary

Tooling may perform these deterministic checks:

* artifact path and identity
* frontmatter schema
* artifact type
* UTF-8 encoding
* LF line endings
* final newline
* content digest
* byte count
* revision handle
* accepted source bindings
* expected target state
* candidate freshness
* lifecycle transition validity
* presence and validity of controlled references
* manifest consistency
* exact-byte equality between approved and accepted content.

> Deterministic support-path validation requires machine-readable references to controlled framing elements. When a support path exists only as free-form prose or quotation, tooling may validate its structure and presence but cannot prove its semantic adequacy.

Deterministic tooling cannot prove semantic adequacy, competency-question
coverage, correctness of cardinality, correctness of domain meaning, whether an
unresolved boundary should be resolved, or whether two differently worded
models are semantically equivalent.

## Semantic Review Boundary

Semantic review determines:

* whether the framing follows from the story
* whether competency questions adequately cover the framing
* whether ontology elements answer those questions
* whether cardinalities are supported
* whether a finding belongs in framing or ontology
* whether a boundary remains unresolved
* whether a candidate adds unsupported meaning
* whether replay output is equivalent, a legitimate alternative, incomplete, or incorrect.

Semantic review must produce structured findings containing:

* finding
* evidence
* affected artifact
* proposed route
* unresolved boundary, if any
* retest requirement.

Allowed routes are:

* revise framing
* revise ontology
* defer
* exclude or
* no change.

Recording a proposed route does not approve the route or the resulting
revision.

## Integrated Audit and Acceptance

After all competency questions have a recorded state, an integrated audit is
required. It must check:

* framing scope
* participants and external dependencies
* competency-question coverage
* concepts
* properties
* relationships
* constraints
* support paths
* unresolved boundaries
* source bindings
* stale-candidate state
* downstream effects.

Acceptance requires:

* deterministic validation passed
* integrated semantic audit completed
* all competency questions classified
* unresolved boundaries preserved or explicitly revised
* no unsupported ontology element accepted
* explicit human approval of one exact staged candidate
* the candidate still current
* accepted bytes equal approved bytes
* the manifest bound to the exact accepted source revisions.

## Open Design Questions

The following remain unresolved implementation questions:

* the final MCP operation names
* request and response schemas
* candidate persistence format
* review-record persistence format
* machine-readable support-reference representation
* whether replacement candidates retain prior review records
* approval granularity
* orchestration policy representation
* replay comparison format
* transaction and recovery implementation.

This document does not resolve these questions.

## Next Design Step

The next design task is to specify the minimum MCP operation schemas and
lifecycle tests needed for one vertical slice:

```text
accepted story
-> staged framing candidate
-> deterministic validation
-> recorded semantic review
-> explicit approval
-> accepted framing
-> one staged ontology candidate
-> one competency-question test
-> replacement and retest if required
-> explicit approval
-> accepted ontology
```

The later replay test must preserve the accepted Raw ADC baseline, create a
separate story-only fixture, run the gated workflow from the accepted story,
compare results semantically against the baseline, classify differences, test
stale-target rejection, test stale-source rejection, test approval enforcement,
and avoid Raw ADC-specific behavior in MCP core.

A second small domain is required before declaring the workflow reusable.
