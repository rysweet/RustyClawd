#!/usr/bin/env python3
"""Quick test of Expert Panel Review pattern.

This script should be run from the orchestration directory or with proper PYTHONPATH.
"""

from patterns import run_expert_panel


def main():
    print("\n" + "=" * 80)
    print("  Expert Panel Review - Quick Test")
    print("=" * 80 + "\n")
    
    # Code with security vulnerability
    code_to_review = """
def authenticate_user(username, password):
    '''Authenticate user and create session.'''
    import sqlite3
    conn = sqlite3.connect('users.db')
    cursor = conn.cursor()
    
    # Query user - VULNERABLE TO SQL INJECTION
    query = f"SELECT * FROM users WHERE username='{username}' AND password='{password}'"
    cursor.execute(query)
    user = cursor.fetchone()
    
    conn.close()
    
    if user:
        return True
    return False
"""
    
    print("Code under review:")
    print(code_to_review)
    print("\nInitiating expert panel review...")
    print("  - Security Expert")
    print("  - Performance Expert")
    print("  - Simplicity Expert")
    print("\nReviewing in parallel (this may take 30-60 seconds)...\n")
    
    # Run expert panel
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
    
    print(f"Session ID: {result['session_id']}")
    print(f"Success: {result['success']}")
    print(f"Reviews Completed: {len(result['reviews'])}/3")
    
    print(f"\n--- FINAL DECISION ---")
    print(f"Decision:   {decision.decision.value.upper()}")
    print(f"Confidence: {decision.confidence:.2f}")
    print(f"Consensus:  {decision.consensus_type}")
    print(f"Agreement:  {decision.agreement_percentage:.1f}%")
    print(f"Quorum Met: {decision.quorum_met}")
    
    print(f"\n--- VOTE BREAKDOWN ---")
    print(f"✓ Approve:  {decision.approve_votes}")
    print(f"✗ Reject:   {decision.reject_votes}")
    print(f"○ Abstain:  {decision.abstain_votes}")
    
    print(f"\n--- INDIVIDUAL REVIEWS ---")
    for i, review in enumerate(result["reviews"], 1):
        vote_symbol = "✓" if review.vote.name == "APPROVE" else ("✗" if review.vote.name == "REJECT" else "○")
        print(f"\n{i}. {review.domain.upper()} Expert: {vote_symbol} {review.vote.value.upper()}")
        print(f"   Confidence: {review.confidence:.2f}")
        print(f"   Rationale: {review.vote_rationale[:200]}")
        if len(review.vote_rationale) > 200:
            print(f"             ...")
        
        if review.strengths:
            print(f"   Strengths ({len(review.strengths)}):")
            for strength in review.strengths[:2]:
                print(f"     + {strength[:100]}")
        
        if review.weaknesses:
            print(f"   Weaknesses ({len(review.weaknesses)}):")
            for weakness in review.weaknesses[:2]:
                print(f"     - {weakness[:100]}")
    
    if result["dissent_report"]:
        print(f"\n--- DISSENT REPORT ---")
        report = result["dissent_report"]
        print(f"Majority Decision: {report.decision.value.upper()} ({report.majority_count} votes)")
        print(f"Dissenting Votes:  {report.dissent_count}")
        print(f"Dissenting Experts: {', '.join(report.dissent_experts)}")
        
        if report.concerns_raised:
            print(f"\nKey Concerns Raised:")
            for concern in report.concerns_raised[:3]:
                print(f"  • {concern[:120]}")
    
    print(f"\n--- SESSION LOGS ---")
    print(f"Full session logs available at:")
    print(f"  .claude/runtime/logs/{result['session_id']}/")
    
    print("\n" + "=" * 80)
    
    # Summary
    if decision.decision.value == "approve":
        print("✓ RESULT: Code APPROVED by expert panel")
    else:
        print("✗ RESULT: Code REJECTED by expert panel")
    
    print("\nExpected outcome: REJECTED due to SQL injection vulnerability")
    print(f"Actual outcome: {decision.decision.value.upper()}")
    
    if decision.decision.value == "reject":
        print("✓ Test PASSED - Security vulnerability correctly identified")
    else:
        print("⚠ Test WARNING - Expected rejection but got approval")
    
    print("=" * 80 + "\n")


if __name__ == "__main__":
    import sys
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
