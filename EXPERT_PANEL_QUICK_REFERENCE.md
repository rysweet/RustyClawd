# Expert Panel Review - Quick Reference

## Overview

**Expert Panel Review** provides Byzantine-robust decision-making through multiple independent expert reviews with vote aggregation.

**Key Features:**
- Parallel independent expert reviews
- Vote-based decisions (APPROVE/REJECT/ABSTAIN)
- Multiple aggregation methods
- Dissent reporting for transparency
- Configurable quorum requirements

## Quick Start

```python
from patterns import run_expert_panel

# Basic usage with default experts
result = run_expert_panel(
    solution="<code or design to review>",
    aggregation_method="simple_majority",
    quorum=3
)

print(f"Decision: {result['decision'].decision.value}")
print(f"Confidence: {result['decision'].confidence:.2f}")
print(f"Votes: {result['decision'].approve_votes}A / {result['decision'].reject_votes}R")
```

## Default Experts (3)

1. **Security**: Vulnerabilities, attack vectors, security best practices
2. **Performance**: Speed, scalability, resource efficiency
3. **Simplicity**: Minimal complexity, maintainability, clarity

## Aggregation Methods

### Simple Majority (Default)
```python
result = run_expert_panel(
    solution=code,
    aggregation_method="simple_majority",
    quorum=3
)
```
- Count votes, majority wins
- Ties default to REJECT (conservative)
- **Use for**: Standard code reviews, general decisions

### Weighted by Confidence
```python
result = run_expert_panel(
    solution=code,
    aggregation_method="weighted",
    quorum=3
)
```
- Weight votes by expert confidence scores
- High-confidence votes carry more weight
- **Use for**: Complex decisions where expertise varies

### Unanimous
```python
result = run_expert_panel(
    solution=critical_code,
    aggregation_method="unanimous",
    quorum=3
)
```
- Require ALL experts to agree
- Any dissent = REJECT
- **Use for**: Security-critical code, production releases

## Custom Expert Panels

```python
custom_experts = [
    {"domain": "security", "focus": "threat modeling, input validation"},
    {"domain": "api_design", "focus": "REST principles, endpoint structure"},
    {"domain": "scalability", "focus": "performance, caching, load handling"},
    {"domain": "compliance", "focus": "regulatory requirements, audit trails"},
]

result = run_expert_panel(
    solution=api_design,
    experts=custom_experts,
    aggregation_method="weighted",
    quorum=3
)
```

## Result Structure

```python
result = {
    "reviews": [ExpertReview, ...],     # All expert reviews
    "decision": AggregatedDecision,      # Final decision
    "dissent_report": DissentReport,     # If dissent exists
    "session_id": "expert-panel-...",    # Session ID for logs
    "success": True                      # Quorum met?
}

# Decision details
decision = result["decision"]
decision.decision         # VoteChoice: APPROVE/REJECT
decision.confidence       # float: 0.0 - 1.0
decision.consensus_type   # "unanimous", "strong_majority", "simple_majority", "split"
decision.approve_votes    # int: number of approve votes
decision.reject_votes     # int: number of reject votes
decision.abstain_votes    # int: number of abstain votes

# Individual review
review = result["reviews"][0]
review.expert_id          # str: expert identifier
review.domain             # str: expert domain
review.vote               # VoteChoice: APPROVE/REJECT/ABSTAIN
review.confidence         # float: expert's confidence (0.0 - 1.0)
review.vote_rationale     # str: why the expert voted this way
review.strengths          # List[str]: identified strengths
review.weaknesses         # List[str]: identified weaknesses
review.analysis           # str: detailed analysis
review.domain_scores      # Dict[str, float]: domain-specific scores
```

## Common Patterns

### Code Review Gate
```python
def code_review_gate(pr_code: str) -> bool:
    """Gate PR merge on expert panel approval."""
    result = run_expert_panel(
        solution=pr_code,
        aggregation_method="simple_majority",
        quorum=3
    )
    
    if not result["success"]:
        print("Quorum not met")
        return False
    
    return result["decision"].decision.value == "approve"
```

### Security Audit (Unanimous)
```python
security_experts = [
    {"domain": "authentication", "focus": "auth mechanisms, sessions"},
    {"domain": "authorization", "focus": "access control, permissions"},
    {"domain": "cryptography", "focus": "encryption, key management"},
    {"domain": "input_validation", "focus": "injection attacks, sanitization"},
]

result = run_expert_panel(
    solution=security_code,
    experts=security_experts,
    aggregation_method="unanimous",  # ALL must approve
    quorum=4
)
```

### With N-Version Programming
```python
# Step 1: Generate multiple implementations
n_version_result = run_n_version(
    task_prompt="Implement password hashing",
    n=3
)

# Step 2: Expert panel reviews each
best_version = None
best_score = 0

for version in n_version_result["versions"]:
    if version.exit_code == 0:
        panel = run_expert_panel(
            solution=version.output,
            aggregation_method="simple_majority",
            quorum=3
        )
        
        score = (panel["decision"].approve_votes, panel["decision"].confidence)
        if score > best_score:
            best_score = score
            best_version = version

print(f"Selected version with {best_score[0]} approvals")
```

## Configuration Options

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `solution` | str | *required* | Code/design to review |
| `experts` | List[Dict] | DEFAULT_EXPERTS | Expert panel definitions |
| `aggregation_method` | str | "simple_majority" | Vote aggregation method |
| `quorum` | int | 3 | Minimum non-abstain votes |
| `model` | str | None | Claude model for experts |
| `working_dir` | Path | current dir | Working directory |
| `timeout` | int | None | Timeout per expert (seconds) |

## Consensus Types

- **unanimous**: 100% agreement
- **strong_majority**: 75%+ agreement
- **simple_majority**: >50% agreement
- **split**: ≤50% agreement (tie)

## Quorum Rules

- Quorum is the **minimum number of non-abstain votes** required
- Abstentions don't count toward quorum
- If quorum not met, `success` = False
- Example: 3 experts, 2 abstain, 1 approve → quorum of 3 NOT met

## When to Use

**Use Expert Panel when you need:**
- Multiple domain perspectives on a decision
- Vote-based decision (not subjective synthesis)
- Protection against individual expert bias
- Clear approval/rejection with rationale
- Transparency in dissenting opinions

**Don't use when:**
- Single expert review is sufficient
- Need iterative refinement (use Debate instead)
- Need multiple solution alternatives (use N-Version instead)

## Pattern Comparison

| Pattern | Purpose | Output |
|---------|---------|--------|
| **Expert Panel** | Evaluate ONE solution from multiple perspectives | Vote decision |
| **N-Version** | Generate MULTIPLE solutions | Best implementation |
| **Debate** | Explore decision through discussion | Synthesized consensus |

**Combine Patterns:** N-Version generates alternatives → Expert Panel selects best

## Session Logs

All reviews and decisions are logged:
```
.claude/runtime/logs/<session_id>/
  orchestrator.log    # Main orchestration log
  expert_security/    # Security expert logs
  expert_performance/ # Performance expert logs
  expert_simplicity/  # Simplicity expert logs
```

## Error Handling

```python
result = run_expert_panel(...)

if not result["success"]:
    if len(result["reviews"]) == 0:
        print("ERROR: All expert reviews failed")
    elif not result["decision"].quorum_met:
        print(f"ERROR: Quorum not met ({len(result['reviews'])} votes)")
else:
    # Process successful decision
    decision = result["decision"]
```

## Examples

Run the demonstration scripts:

```bash
# Quick test with simple code review
python test_expert_panel.py

# Interactive demo with multiple examples
python demo_expert_panel.py
```

## Full Documentation

See comprehensive documentation:
- Pattern details: `.claude/commands/amplihack/expert-panel.md`
- Implementation: `.claude/tools/amplihack/orchestration/patterns/expert_panel.py`
- Examples: `demo_expert_panel.py`

---

**Pattern**: Expert Panel Review  
**Status**: Production Ready  
**Version**: 1.0
