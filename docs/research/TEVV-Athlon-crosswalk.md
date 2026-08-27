# TEVV-Athlon Crosswalk for Precision Replay and RMWM

**Source:** P. Jonathon Phillips et al., *The TEVV-Athlon Framework for Evaluating AI Systems*, NIST AI 200-2 ipd, Initial Public Draft, August 2026. [NIST PDF](https://nvlpubs.nist.gov/nistpubs/ai/NIST.AI.200-2.ipd.pdf) | [DOI](https://doi.org/10.6028/NIST.AI.200-2.ipd)

**Status:** Working interpretation for our current program. The NIST public-comment period closes **October 6, 2026**.

## Authority and use

This document is a research crosswalk and external-framework interpretation.

It is not:
- an accepted RMWM managed artifact
- an authoritative ontology
- an implementation contract
- a requirements source
- evidence that RMWM or Precision Replay conforms to TEVV-Athlon

Its purpose is to identify useful correspondences, gaps, research questions, and candidate future domain work.

## Bottom line

TEVV-Athlon gives us a credible external frame for designing a tailored evaluation:

> organizational goal -> system attribute or trustworthiness characteristic -> Metrology Block -> Event -> Toolbox -> evidence -> synthesis and decision

Our work is exploring a complementary evidentiary spine:

> explicit claim -> exact evidence requirement -> identified event and tool execution -> retained source record -> traceable transformation -> canonical replay input -> deterministic execution record -> bounded comparison -> justified and unjustified conclusions

The two are complementary. TEVV-Athlon tells an evaluator **what assessment to construct and why**. Precision Replay and the requirements-model workflow address **how the resulting evidence and conclusions remain inspectable, revision-bound, and replayable**.

The strongest research intersection is not "we implement TEVV-Athlon." It is:

> A TEVV design is only as credible as the trace from its stated measurement concept to the exact evidence, transformations, executions, and claim boundaries supporting its conclusions.

## The framework in one screen

| Stage | NIST question | Main outputs |
| --- | --- | --- |
| 1. Articulate & Organize | Why evaluate, for whom, at what lifecycle stage, and against which organizational goals? | Evaluation goal, scope, stakeholders, relevant system attributes or trustworthiness characteristics, rough resources and constraints |
| 2. Define & Construct | What concept or metric will represent the attribute of interest? | One or more precisely defined **Metrology Blocks**, including the kinds of evidence required |
| 3. Apply & Measure | What activities will produce that evidence, and by what methods or instruments? | **Events** and a **Toolbox** used to elicit, collect, and analyze evidence |
| 4. Synthesize & Interrogate | What does the collected evidence show, how valid is the measurement, and what decision should it inform? | Joint analysis, limitations, conclusions, report, and input to risk management |

NIST deliberately makes this flexible. It also says the measurement method must itself be evaluated, that controlled tests should be complemented by realistic testing, that assumptions, limitations, variability, uncertainty, baselines, variables, conditions, and procedures should be documented, and that measurement validity must be revisited over time.

## Interpretive crosswalk to current work

| TEVV-Athlon concept | Our nearest artifact or operation | Fit | Important difference |
| --- | --- | --- | --- |
| Organizational goals and operating context | System story and domain framing | Strong | Our framing explicitly records included, excluded, and deferred questions, which helps bound later conclusions. |
| Attribute or trustworthiness characteristic | Claim family or property of interest | Partial | An attribute is still broad. Our central unit should remain the **claim that the evidence may justify**. |
| Metrology Block | Controlled concept plus evidence requirements | Strong | NIST requires a precise definition and required evidence. We additionally need version identity and explicit links to the claims the Block can and cannot support. |
| Event | Acquisition, projection, validation, replay execution, functional comparison, timing evaluation | Strong | We distinguish an Event definition from an actual Event run and its retained record. The draft mostly discusses Event types. |
| Toolbox | ADC acquisition tooling, projection code, validators, replay runner, comparison procedures | Strong | Tool name is insufficient. We need executable version, configuration, inputs, environment, and output bindings. |
| Collected evidence | Raw ADC records, acquisition context, canonical replay input, execution record, comparison result | Very strong | We preserve evidence lineage across intentional reductions rather than treating collected data as a flat result set. |
| Synthesize & Interrogate | Claim-bounded evidence package and independent inspection | Strong | We require the report to state both what remains justified and what does not. |
| Measurement validation | Functional comparison separated from timing, target-context, and physical-measurement evaluation | Very strong | Our separation prevents success in one claim family from silently standing in for another. |
| Reproducibility guidance | Deterministic replay and portable evidence package | Very strong | NIST asks for sufficient documentation "where feasible." We are exploring stronger machine-verifiable bindings and deterministic re-execution. |
| Periodic review | Revision freshness, stale/review-required state, candidate/acceptance lifecycle | Strong | RMWM makes dependency change and review state explicit instead of relying only on process discipline. |

## Raw-ADC proof case expressed as a TEVV-Athlon

The translation below is intentionally claim-bounded: it uses TEVV-Athlon to describe an evaluation design while retaining the accepted Raw-ADC artifacts as the authority for domain meaning.

### Articulate & Organize

**Goal:** Determine whether a canonical projection of retained raw ADC records preserves enough information to support a specified set of deterministic functional replay claims.

**Primary audience:** System developers, evaluators, and independent inspectors who need to know what the replay result does and does not establish.

**Claim boundary:** The assessment may support functional equivalence claims defined over the canonical input and deterministic operation. It does not automatically support claims about physical measurement truth, sampling fidelity, target timing, or deployment-context behavior.

### Define & Construct

Candidate Metrology Blocks:

1. **Projection validity** - whether an accepted raw record is transformed into canonical replay input according to the declared projection and validation rules.
2. **Functional replay agreement** - whether deterministic execution over the canonical input produces an execution record that satisfies the declared comparison relation.
3. **Evidence trace completeness** - whether an inspector can follow immutable identities and bindings from retained acquisition evidence through projection, execution, and comparison.
4. **Claim-boundary completeness** - whether the resulting package states which claim families are supported, unsupported, or evaluated separately.

### Apply & Measure

| Event | Toolbox elements | Principal retained evidence |
| --- | --- | --- |
| Acquire and decide | ADC capture procedure, acceptance/rejection rule | Raw ADC record, acquisition context, disposition and reason |
| Select and project | Selection rule, projection implementation | Selected record identity, projection version/configuration, canonical input, source-to-output binding |
| Validate canonical input | Validator and declared schema/invariants | Validation result and failure reason |
| Execute replay | Deterministic runner and operation version | Execution record bound to canonical input and executable identity |
| Compare functionally | Declared comparator and functional reference | Comparison result, relation used, mismatch reason |
| Evaluate timing/context separately | Target-specific instrumentation and procedures | Timing or context evidence that cannot be inferred from functional agreement |
| Assemble package | Manifest and integrity tooling | Portable evidence package with digests, provenance, dependencies, limitations, and claim scope |

### Synthesize & Interrogate

The synthesis should answer four separate questions:

1. Did the declared projection produce valid canonical input from the identified retained source?
2. Did deterministic execution satisfy the declared functional comparison?
3. Can an independent inspector verify the full evidence chain without relying on an unrecorded step?
4. Which conclusions are warranted, and which remain outside the evidence package?

## What TEVV-Athlon adds to our program

1. **A recognized outer vocabulary.** "Goal, Block, Event, Toolbox, evidence, synthesis" gives us a way to explain the evaluation construction without leading with our internal machinery.
2. **Measurement validity as a first-class problem.** The draft explicitly says evaluators must assess whether a measurement actually captures the intended concept and whether important parts of the problem are missing.
3. **Evaluation of the evaluator.** Tools, prompts, benchmarks, rubrics, and procedures are themselves measurement instruments whose quality must be assessed.
4. **Controlled plus operational evidence.** NIST warns that controlled model tests may not predict deployed behavior. This reinforces our existing separation of deterministic functional evidence from timing and target-context evidence.
5. **Baselines, uncertainty, variables, and conditions.** These should appear explicitly in future evaluation packages even when deterministic replay makes run-to-run execution variance zero. Determinism does not eliminate uncertainty in acquisition, projection adequacy, reference choice, or claim interpretation.
6. **Goodhart pressure.** A score may become a target and drift away from the real property of interest. Our defense should be plural evidence, explicit claim scope, retained source material, and the ability to revise a projection or measurement definition without erasing prior evaluation identity.

## Traceability details not specified by this draft

The draft is a design framework, not an evidence interchange or provenance standard. It does not yet require a precise representation for:

- the identity of an Event definition versus a particular Event run
- the exact version and configuration of each Tool
- immutable bindings among source data, derived data, executions, and reports
- transformation lineage when evaluation evidence is reduced, normalized, annotated, or projected
- the claim or conclusion each evidence item is entitled to support
- explicit unsupported or excluded claim families
- dependency freshness when goals, Blocks, Events, Tools, or evidence change
- deterministic or independently repeatable re-execution
- machine-checkable completeness of the path from goal to conclusion

This is the most important overlap with claim-preserving replay projection. The scientific question appears inside TEVV whenever an evaluator transforms rich observations into a smaller evaluation representation:

> What information may be discarded while preserving the claims the evaluation is meant to support?

## Research questions produced by the crosswalk

The crosswalk raises several questions that should remain open until tested:

1. What is the smallest provenance structure sufficient to bind a TEVV conclusion to the exact evidence and transformations supporting it?

2. When evidence is projected, normalized, summarized, or otherwise reduced, what relation must hold between the source representation and the derived representation for a specified claim to remain justified?

3. Which upstream changes should cause an existing evaluation or conclusion to become stale, review-required, or superseded, and under what conditions does that change actually invalidate the conclusion?

4. Can evaluation replay be defined independently of deterministic system replay, or is a broader notion of claim-preserving evaluation reconstruction needed?

5. What evidence is required to distinguish:
   - repeatability of an execution
   - reproducibility of an evaluation
   - validity of a measurement
   - preservation of a claim across abstraction?

6. How much of this trace can be machine-checked without embedding domain judgment into the workflow kernel?

These questions are research inputs, not current requirements.

## Implications for RMWM

Do **not** import TEVV-Athlon wholesale into the generic lifecycle kernel before the MCP release candidate. The kernel should continue to enforce identity, revision, staging, validation, approval, freshness, dependencies, readback, and recoverable failure. It must not pretend to prove domain meaning.

After the RC, use TEVV-Athlon as a bounded domain-profile test:

1. Model the TEVV-Athlon vocabulary through the normal story -> framing -> ontology -> controlled vocabulary -> requirements workflow.
2. Keep **Event definition/Event run**, **Tool specification/Tool run**, and **evidence item/derived result** distinct.
3. Test whether an accepted upstream change correctly marks dependent evaluation artifacts stale or review-required.
4. Require human review of Block meaning, evidence adequacy, comparison choice, and claim scope.
5. Use Raw ADC as the proof case and an AI evaluation case later as the transfer test. This avoids overfitting the workflow to embedded replay.

Candidate concepts for later domain work, subject to framing and competency questions:

- EvaluationGoal
- StakeholderDecision
- SystemAttribute
- TrustworthinessCharacteristic
- MetrologyBlock
- EvidenceRequirement
- EventDefinition and EventRun
- ToolSpecification and ToolRun
- EvidenceItem and DerivedResult
- Baseline
- Variable and Condition
- UncertaintyStatement
- Synthesis
- Claim, ClaimBoundary, and Limitation
- ProvenanceBinding

These are ontology candidates, not immediate implementation requirements.

## Recommended NIST comment focus

This draft creates a better comment opportunity than a broad argument about replay. A narrow, constructive comment could recommend that the framework make the following trace explicit:

> goal -> attribute/characteristic -> Block -> evidence requirement -> Event -> Tool execution -> evidence item -> derived result -> conclusion

The comment can then request guidance or normative language for:

1. distinguishing definitions from executions and observations
2. recording tool, data, configuration, and environment versions
3. preserving provenance across transformations and reductions
4. binding conclusions to the exact evidence that supports them
5. stating unsupported or out-of-scope claims alongside supported claims
6. detecting when upstream changes invalidate or stale downstream results
7. enabling independent verification or replay where feasible

That comment is fully aligned with the paper's own emphasis on evidence-based claims, reproducibility, limitations, baselines, uncertainty, measurement validity, and independent review. It extends the framework at its traceability seam rather than asking NIST to adopt our entire architecture.

## Sequencing

1. **Now:** Treat this crosswalk as research input. Do not let it interrupt the MCP RC.
2. **MCP RC:** Complete the end-to-end story -> framing -> ontology -> vocabulary -> requirements path, deterministic candidate/commit separation, reliable human review, immutable digests and bindings, clean readback, provenance, recoverable failures, contract tests, runnable documentation, and the Raw-ADC proof case.
3. **After RC:** Run TEVV-Athlon as a domain-profile exercise and identify the smallest credible NIST comment.
4. **Before October 6, 2026:** Submit a concise comment centered on traceable, replayable, claim-bounded evaluation evidence.

## Retained thesis

**TEVV-Athlon organizes an evaluation around goals, measurement concepts, events, tools, and evidence. Our research direction is to make the path from those elements to a conclusion claim-relative, transformation-aware, revision-bound, and independently replayable.**
