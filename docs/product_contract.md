# Requirements Model Workflow MCP Product Contract

Provenance: `RMWM-PROVENANCE-2026-08-07-01`

Status: Initial draft

## Purpose

Requirements Model Workflow MCP guides the development of a natural-language system story into:

domain framing
→ domain ontology
→ controlled vocabulary
→ requirements
→ requirement decomposition
→ traceability

The product is reusable across engineering domains. Precision Replay will be its first substantial reference case, not part of its generic architecture.

## Working Principle

The workflow develops one evolving requirements model organized around a domain ontology.

The ontology identifies the concepts, relationships, properties, and constraints used to describe the domain. It provides the semantic connection between the story, controlled vocabulary, requirements, decomposition, and traceability.

Each stage adds to or refines this shared model. Later stages reference accepted ontology elements rather than independently reinterpreting the original story.

The ontology is a first-class workflow artifact. It is reviewed, versioned, and approved like the requirements derived through it.

## Responsibility Boundary

Humans decide what the system means and approve changes.

Agents may propose:

* boundaries
* competency questions
* concepts and relationships
* definitions
* requirements
* decomposition
* trace links

The MCP owns:

* workflow state
* candidate staging
* source and candidate identity
* approval gates
* structural validation
* reference validation
* accepted-model persistence
* deterministic projections

Generated content has no authority until explicitly approved.

## Workflow

### Domain Framing

Establishes scope, exclusions, participants, external dependencies, and the questions the model must answer.

### Domain Ontology

Establishes the concepts, relationships, properties, and constraints needed to describe the domain and address the accepted competency questions.

Each accepted ontology element receives a stable identity that may be referenced by vocabulary entries, requirements, decomposition links, and traceability records.

The ontology may evolve as the requirements develop. Changes to accepted ontology elements identify dependent work requiring review.

### Controlled Vocabulary

Assigns preferred terms and definitions to accepted ontology elements.

The vocabulary is a human-readable view of the ontology rather than an independent interpretation of the story.

### Requirements

Defines normative obligations over accepted ontology elements.

Requirement prose remains normative. Ontology references establish the concepts and relationships constrained by that prose.

### Requirement Decomposition

Relates parent requirements to more specific requirements without assuming one universal HLR/LLR structure.

### Traceability

Records typed links among sources, ontology elements, vocabulary terms, requirements, decomposition, implementation references, verification references, and evidence references.

## Review Lifecycle

Each stage follows:

accepted model
→ staged candidate
→ human review
→ optional revision
→ explicit approval and commit
→ accepted model revision

Generation and commit are separate operations.

A staged candidate is bound to the source and model revisions from which it was prepared. A stale candidate cannot be committed.

Earlier model elements may be revised. Dependent work is marked for review rather than silently deleted or rewritten.

## Determinism Boundary

Mechanical work belongs in deterministic tooling. This includes identifiers, hashes, structural checks, reference checks, persistence, dependency tracking, and traceability projection.

Semantic interpretation belongs to a human or agent and remains subject to review.

## Ontology Representation

The domain ontology is a first-class part of the accepted requirements model. It is independent of any one storage or presentation format.

Markdown, JSON, RDF, OWL, diagrams, requirements tables, and traceability matrices may be supported as projections or adapters.

The initial implementation will not assume that RDF or OWL must be the internal storage format. Representation choices shall not weaken the ontology’s role in the workflow.

## Product Exclusions

The product is not:

* an autonomous requirements author
* a general Markdown framework
* a requirements database
* a verification or proof system
* a certification authority
* specific to Precision Replay

## First Milestone

The first milestone will exercise the complete workflow with:

1. a small domain-neutral story
2. the Precision Replay story as the first substantial reference case

The milestone succeeds when each case can develop a reviewed model from story through traceability without adding domain-specific behavior to the workflow core.
