# Requirements Model Workflow MCP Product Contract

Provenance: `RMWM-PROVENANCE-2026-08-07-01`

Status: Initial draft

## Purpose

Requirements Model Workflow MCP guides the development of a natural-language
system story through:

domain framing
→ domain ontology
→ controlled vocabulary
→ requirements
→ requirement decomposition
→ traceability

The product is reusable across engineering domains. Precision Replay will serve
as its first substantial reference case. It is not part of the generic
architecture.

The workflow produces a reviewed requirements model containing an accepted
domain ontology, a controlled vocabulary, ontology-linked requirements,
requirement decomposition, and traceability.

## Working Principle

The workflow develops one evolving requirements model organized around a
domain ontology.

The ontology identifies the concepts, relationships, properties, and
constraints used to describe the domain. It provides the semantic connection
between the story, controlled vocabulary, requirements, decomposition, and
traceability.

Each stage adds to or refines this shared model. Later stages reference
accepted ontology elements rather than independently reinterpreting the
original story.

The ontology is a first-class workflow artifact. It is reviewed, versioned, and
approved like the requirements derived through it.

## Minimum Completion and Ontology Invariants

The workflow is complete only when:

* every workflow stage has an accepted result
* each competency question is addressed, deferred, or excluded
* each vocabulary entry references an accepted ontology element
* each requirement references accepted ontology elements
* required trace links connect accepted elements
* unresolved items remain explicit
* ontology element identities remain stable
* changes to accepted ontology elements identify dependent work that requires
  review

An accepted stage result may state that no additional content is required. The
result shall identify the accepted predecessor elements considered, provide a
rationale, satisfy all applicable completion invariants, and receive explicit
human approval. A change to a predecessor requires review of this
no-additional-content outcome.

Every accepted ontology element shall have a support path to the accepted
domain framing. The path may pass through other accepted ontology elements and
shall terminate at an included concern, a participant, an external dependency,
or a competency question that remains in scope. Circular ontology references
do not provide sufficient support. An excluded concern or competency question
cannot support an accepted ontology element unless the domain framing is
revised and approved. An unresolved boundary question cannot provide accepted
support unless it is resolved into an accepted in-scope framing element. A
proposed ontology element without a support path shall be rejected or shall
cause an approved revision to the domain framing.

## Responsibility Boundary

Humans decide what the system means and approve changes.

Agents may propose:

* boundaries
* competency questions
* concepts, relationships, properties, and constraints
* preferred terms and definitions
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
* persistence of the accepted workflow model and workflow state
* deterministic projections

This persistence does not replace external project artifacts that the project
identifies as authoritative.

## Managed-Artifact State Contract

The body of an artifact owns domain meaning. Artifact frontmatter owns stable
artifact identity and type. The requirements-model manifest owns accepted
revisions and source bindings. The MCP owns active stage and staged-candidate
lifecycle state. Deterministic calculation owns modified, stale,
review-required, and superseded state.

Managed artifact frontmatter has this closed structure:

```yaml
---
rmwm:
  schema: "artifact/v1"
  id: "<artifact-id>"
  type: "<artifact-type>"
---
```

The `rmwm` object is closed; unknown fields are invalid. Controlled artifact
types are `system_story`, `domain_framing`, `domain_ontology`,
`controlled_vocabulary`, `requirements`, `requirement_decomposition`, and
`traceability`.

The requirements-model manifest has the closed schema
`rmwm/requirements-model/v1`. Each artifact has exactly one representation
location. Repository-local paths are normalized relative to the manifest and
cannot use `..`, absolute roots, or symlink indirection. A future external
artifact may use `uri` instead of `path`; no external artifact is used here.

Content identity is the SHA-256 digest and exact byte count of the complete
file, including frontmatter. Content is UTF-8 without a BOM and declares its
line endings.

A revision handle identifies content together with its accepted source
revisions. It is calculated deterministically from the revision schema,
model ID, artifact ID, artifact type, workflow stage, complete content digest,
source count, and source artifact ID plus accepted revision pairs. Strings are
encoded as an unsigned 64-bit big-endian UTF-8 byte length followed by their
UTF-8 bytes; source count is an unsigned 64-bit big-endian integer; source
pairs are sorted lexicographically by source artifact ID. The concatenated
bytes are hashed with SHA-256 using revision schema `rmwm-revision-v1`.

Derived state is not stored in artifact frontmatter:

* `draft`: `accepted` is null.
* `accepted`: file identity and type match the manifest, and exact bytes match
  the accepted content descriptor.
* `modified`: current file bytes differ from the accepted content descriptor.
* `staged`: matching staged-candidate state exists in MCP persistence.
* `stale`: the accepted target revision or a bound source revision changed
  after staging.
* `review_required`: an accepted artifact's bound source revision differs
  from the current accepted source revision.
* `superseded`: a later accepted revision replaced an earlier accepted revision
  for the same artifact.

### Candidate Contract

The future MCP behavior is:

1. The agent supplies the candidate body.
2. The MCP owns and validates controlled frontmatter.
3. The MCP applies the declared encoding and line endings.
4. The MCP constructs the exact prospective artifact bytes.
5. The MCP calculates the content digest and revision handle.
6. The staged candidate binds the model ID, artifact ID, artifact type,
   workflow stage, current accepted target revision if one exists, exact
   prospective artifact bytes, and accepted source revisions.
7. Commit rejects a changed target or source.
8. Commit writes the exact bytes that were staged.

Generation and commit remain separate operations.

### Transaction Contract

Commit is one logical transaction:

1. Lock the requirements model.
2. Re-read and validate the manifest and accepted artifacts.
3. Revalidate the candidate handle, target base revision, and source revisions.
4. Prepare the new artifact and manifest in temporary files.
5. Preserve recoverable copies of the old artifact and manifest.
6. Persist a recovery journal containing target paths and old and new
   descriptors.
7. Persist the temporary files.
8. Replace the artifact.
9. Replace the manifest last.
10. Treat manifest replacement as the acceptance point.
11. Clear staged-candidate state.
12. Remove recovery files only after completion.

Recovery rules:

* If the old manifest remains, restore the old artifact.
* If the new manifest is present and valid, retain the new artifact and finish
  clearing candidate state.
* For any unresolved digest or identity mismatch, stop without guessing.

Generated content has no authority until explicitly approved.

## Workflow

### Domain Framing

Domain framing establishes the domain scope by identifying included and
excluded concerns, participants, external dependencies, and the competency
questions the domain ontology must address.

### Domain Ontology

Establishes the concepts, relationships, properties, and constraints needed to
describe the domain and address the accepted competency questions.

Each accepted ontology element receives a stable identity that may be referenced
by vocabulary entries, requirements, decomposition links, and traceability
records.

The ontology may evolve as the requirements develop. Changes to accepted
ontology elements identify dependent work requiring review.

### Controlled Vocabulary

Assigns preferred terms and definitions to accepted ontology elements.

The vocabulary is a human-readable view of the ontology rather than an
independent interpretation of the story.

### Requirements

Defines normative obligations over accepted ontology elements.

Requirement prose remains normative. Ontology references identify the elements
that the requirement constrains.

### Requirement Decomposition

Relates parent requirements to more specific requirements without prescribing a
universal set of requirement levels.

Each accepted requirement shall have one or more accepted child requirements or
an explicit and approved no-further-decomposition outcome. The guarded
no-additional-content rules above apply to that outcome. Each child requirement
shall reference at least one accepted parent requirement and accepted ontology
elements. Decomposition shall contain no cycles. Human review determines
whether each child preserves and refines its parent's meaning.

### Traceability

Records typed links among sources, ontology elements, vocabulary terms,
requirements, decomposition, implementation references, verification
references, and evidence references.

At workflow completion, each accepted requirement shall have a trace path
through accepted ontology elements to the accepted domain framing. Each child
requirement shall trace to an accepted parent requirement.

Implementation, verification, and evidence links are optional. Each link that
exists shall connect an accepted workflow element to a structurally valid
external reference.

## Review Lifecycle

Each stage follows:

accepted source or model revision
→ staged candidate
→ human review
→ optional revision
→ explicit approval and commit
→ accepted model revision

A stage may produce an accepted revision that permits work on a later stage. Acceptance does not close the stage. Findings from later stages may require a new revision of an earlier stage and review of dependent work.

Generation and commit are separate operations.

A staged candidate is bound to the applicable source and model revisions used to
prepare it. A stale candidate cannot be committed.

A staged candidate may be approved, rejected, replaced, or withdrawn. Only an
approved and committed candidate becomes part of the accepted model. A
replacement identifies the candidate it supersedes.

Earlier model elements may be revised. Dependent work is marked for review
rather than silently deleted or rewritten.

## Determinism Boundary

Mechanical work belongs in deterministic tooling. This includes identifiers,
hashes, structural checks, reference checks, persistence, dependency tracking,
and traceability projection.

Semantic interpretation belongs to a human or agent and remains subject to
review.

## Ontology Representation

The domain ontology is a first-class part of the accepted requirements model.
It is independent of any one storage or presentation format.

Markdown, JSON, RDF, OWL, diagrams, requirements tables, and traceability
matrices may be supported as projections or adapters.

The initial implementation will not assume that RDF or OWL must be the internal
storage format. Representation choices shall not weaken the ontology's role in
the workflow.

## Product Exclusions

The product is not:

* an autonomous requirements author
* a general Markdown framework
* a general-purpose requirements database
* a verification or proof system
* a certification authority
* specific to Precision Replay

## Milestones

### Milestone 1: Raw-ADC Walking Skeleton

The raw-ADC capture example shall exercise the complete workflow. It must
demonstrate:

* approval and commit of a valid candidate
* rejection of an invalid reference
* rejection of a stale candidate
* revision of an accepted ontology element
* identification of downstream work that requires review

The milestone succeeds when the case can develop a reviewed model from the
raw-ADC story through traceability without adding domain-specific behavior to
the workflow core.

### Milestone 2: Precision Replay Reference Validation

Precision Replay shall complete the same workflow and preserve the generic
workflow stages. The workflow core shall contain no Replay-specific behavior.

This milestone shall evaluate whether the more complex domain exposes generic
limitations and record the result.

The product does not establish reusable adequacy until both milestones pass.
