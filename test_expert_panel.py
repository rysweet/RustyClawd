#!/usr/bin/env python3
"""Quick test of Expert Panel Review pattern with a simple code review."""

import sys
from pathlib import Path

# Add orchestration patterns to path
sys.path.insert(0, str(Path(__file__).parent / ".claude/tools/amplihack/orchestration"))

from patterns import run_expert_panel


def main():
    print("\n" + "=" * 80)
    print("  Expert Panel Review - Quick Test")
    print("=" * 80 + "\n")
    
    # Code with obvious security issue
    code_to_review = """
def get_user_by_id(user_id):
    '''Fetch user from database by ID.'''
    import sqlite3
    conn = sqlite3.connect('app.db')
    cursor = conn.cursor()
    
    # Fetch user
    query = f"SELECT * FROM users WHERE id = {user_id}"
    cursor.execute(query)
    user = cursor.fetchone()
    
    conn.close()
    return user
"""
    
    print("Code under review:")
    print(code_to_review)
    print("\nInitiating expert panel review with 3 default experts...")
    print("  - Security Expert")
    print("  - Performance Expert")
    print("  - Simplicity Expert")
    print("\nThis will take approximately 30-60 seconds...\n")
    
    # Run expert panel with simple majority
    result = run_expert_panel(
        solution=code_to_review,
        aggregation_method="simple_majority",
        quorum=3
    )
    
    # Print results
    print("\n" + "=" * 80)
    print("  RESULTS")
    print("=" * 80 + "\n")
    
    decision = result["decision"]
    
    print(f"✓ Session ID: {result['session_id']}")
    print(f"✓ Success: {result['success']}")
    print(f"✓ Reviews Completed: {len(result['reviews'])}/{3}")
    
    print(f"\n--- FINAL DECISION ---")
    print(f"Decision:   {decision.decision.value.upper()}")
    print(f"Confidence: {decision.confidence:.2f}")
    print(f"Consensus:  {decision.consensus_type}")
    print(f"Agreement:  {decision.agreement_percentage:.1f}%")
    
    print(f"\n--- VOTE BREAKDOWN ---")
    print(f"✓ Approve:  {decision.approve_votes}")
    print(f"✗ Reject:   {decision.reject_votes}")
    print(f"○ Abstain:  {decision.abstain_votes}")
    
    print(f"\n--- INDIVIDUAL REVIEWS ---")
    for i, review in enumerate(result["reviews"], 1):
        vote_symbol = "✓" if review.vote.name == "APPROVE" else ("✗" if review.vote.name == "REJECT" else "○")
        print(f"\n{i}. {review.domain.upper()} Expert: {vote_symbol} {review.vote.value.upper()}")
        print(f"   Confidence: {review.confidence:.2f}")
        print(f"   Rationale: {review.vote_rationale[:150]}...")
        
        if review.strengths:
            print(f"   Strengths ({len(review.strengths)}):")
            for strength in review.strengths[:2]:
                print(f"     + {strength[:80]}")
        
        if review.weaknesses:
            print(f"   Weaknesses ({len(review.weaknesses)}):")
            for weakness in review.weaknesses[:2]:
                print(f"     - {weakness[:80]}")
    
    if result["dissent_report"]:
        print(f"\n--- DISSENT REPORT ---")
        report = result["dissent_report"]
        print(f"Majority Decision: {report.decision.value.upper()} ({report.majority_count} votes)")
        print(f"Dissenting Votes:  {report.dissent_count}")
        print(f"Dissenting Experts: {', '.join(report.dissent_experts)}")
        
        if report.concerns_raised:
            print(f"\nKey Concerns Raised:")
            for concern in report.concerns_raised[:3]:
                print(f"  • {concern[:100]}")
    
    print(f"\n--- SESSION LOGS ---")
    print(f"Full session logs available at:")
    print(f"  .claude/runtime/logs/{result['session_id']}/")
    
    print("\n" + "=" * 80)
    print("  Test Complete")
    print("=" * 80 + "\n")
    
    # Summary
    if decision.decision.value == "approve":
        print("✓ RESULT: Code APPROVED by expert panel")
    else:
        print("✗ RESULT: Code REJECTED by expert panel")
    
    print(f"\nFor full documentation, see:")
    print(f"  .claude/commands/amplihack/expert-panel.md")
    print()


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n\nInterrupted by user. Exiting...")
        sys.exit(1)
    except Exception as e:
        print(f"\n\nERROR: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
