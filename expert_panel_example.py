#!/usr/bin/env python3
"""Quick demonstration of Expert Panel Review pattern - Code Review Use Case.

This shows a practical example of using expert panel to review a password hashing
implementation with three domain experts voting on approval.
"""

import sys
from pathlib import Path

# Add orchestration module to path
sys.path.insert(0, str(Path(__file__).parent / ".claude/tools/amplihack"))

from orchestration.patterns.expert_panel import run_expert_panel, VoteChoice


def main():
    print("\n" + "=" * 80)
    print("EXPERT PANEL REVIEW: Password Hashing Implementation")
    print("=" * 80 + "\n")

    # Code to review
    code_implementation = """
import hashlib
import os
from typing import Tuple

def hash_password(password: str, iterations: int = 100000) -> Tuple[str, str]:
    '''
    Hash a password using PBKDF2-HMAC-SHA256.
    
    Args:
        password: Plain text password to hash
        iterations: Number of iterations (default 100,000)
    
    Returns:
        Tuple of (password_hash, salt) as hex strings
    '''
    # Generate random 32-byte salt
    salt = os.urandom(32)
    
    # Hash password with PBKDF2
    pwdhash = hashlib.pbkdf2_hmac(
        'sha256',
        password.encode('utf-8'),
        salt,
        iterations
    )
    
    return pwdhash.hex(), salt.hex()


def verify_password(stored_hash: str, stored_salt: str, 
                   password: str, iterations: int = 100000) -> bool:
    '''
    Verify a password against stored hash and salt.
    
    Args:
        stored_hash: Hex-encoded password hash
        stored_salt: Hex-encoded salt
        password: Plain text password to verify
        iterations: Number of iterations used in hashing
    
    Returns:
        True if password matches, False otherwise
    '''
    salt = bytes.fromhex(stored_salt)
    
    # Compute hash of provided password
    pwdhash = hashlib.pbkdf2_hmac(
        'sha256',
        password.encode('utf-8'),
        salt,
        iterations
    )
    
    # Constant-time comparison
    return pwdhash.hex() == stored_hash


# Example usage
if __name__ == "__main__":
    # Hash a password
    password = "MySecurePassword123!"
    hash_value, salt = hash_password(password)
    print(f"Hash: {hash_value[:32]}...")
    print(f"Salt: {salt[:32]}...")
    
    # Verify correct password
    assert verify_password(hash_value, salt, password) == True
    
    # Verify incorrect password
    assert verify_password(hash_value, salt, "WrongPassword") == False
    
    print("All tests passed!")
"""

    print("Code under review:")
    print("-" * 80)
    print(code_implementation[:500] + "...")
    print("-" * 80)

    print("\nInitiating Expert Panel Review...")
    print("• Security Expert: Analyzing vulnerabilities and attack vectors")
    print("• Performance Expert: Evaluating efficiency and scalability")
    print("• Simplicity Expert: Assessing maintainability and clarity")
    print()

    # Run expert panel with default experts
    result = run_expert_panel(
        solution=code_implementation,
        aggregation_method="simple_majority",
        quorum=3,
        timeout=120  # 2 minutes per expert
    )

    # Print results
    print("\n" + "=" * 80)
    print("REVIEW RESULTS")
    print("=" * 80)

    if not result['success']:
        print("\n❌ Review FAILED - Quorum not met")
        print(f"Reviews completed: {len(result['reviews'])}")
        return

    decision = result['decision']
    
    # Decision header with visual indicator
    if decision.decision == VoteChoice.APPROVE:
        indicator = "✅ APPROVED"
        color = "🟢"
    else:
        indicator = "❌ REJECTED"
        color = "🔴"
    
    print(f"\n{color} {indicator}")
    print(f"   Confidence: {decision.confidence:.2%}")
    print(f"   Consensus: {decision.consensus_type.replace('_', ' ').title()}")
    print(f"   Agreement: {decision.agreement_percentage:.1f}%")
    
    # Vote tally
    print(f"\n📊 Vote Breakdown:")
    print(f"   ✓ Approve: {decision.approve_votes}")
    print(f"   ✗ Reject:  {decision.reject_votes}")
    print(f"   − Abstain: {decision.abstain_votes}")
    
    # Individual expert reviews
    print(f"\n{'─' * 80}")
    print("EXPERT REVIEWS")
    print('─' * 80)

    for review in result['reviews']:
        # Vote emoji
        if review.vote == VoteChoice.APPROVE:
            vote_emoji = "✅"
        elif review.vote == VoteChoice.REJECT:
            vote_emoji = "❌"
        else:
            vote_emoji = "−"
        
        print(f"\n{vote_emoji} {review.domain.upper()} EXPERT")
        print(f"   Vote: {review.vote.value.upper()} (confidence: {review.confidence:.2%})")
        
        print(f"\n   Rationale:")
        # Wrap rationale text
        import textwrap
        wrapped = textwrap.fill(review.vote_rationale, width=72, 
                               initial_indent='   ', subsequent_indent='   ')
        print(wrapped)
        
        if review.strengths:
            print(f"\n   ✓ Key Strengths:")
            for strength in review.strengths[:2]:
                wrapped = textwrap.fill(f"• {strength}", width=72,
                                      initial_indent='     ', subsequent_indent='       ')
                print(wrapped)
        
        if review.weaknesses:
            print(f"\n   ✗ Key Weaknesses:")
            for weakness in review.weaknesses[:2]:
                wrapped = textwrap.fill(f"• {weakness}", width=72,
                                      initial_indent='     ', subsequent_indent='       ')
                print(wrapped)
        
        if review.domain_scores:
            print(f"\n   📈 Domain Scores:")
            for aspect, score in list(review.domain_scores.items())[:3]:
                bar_length = int(score * 20)
                bar = "█" * bar_length + "░" * (20 - bar_length)
                print(f"     {aspect:20s} [{bar}] {score:.2f}")

    # Dissent report
    if result['dissent_report']:
        report = result['dissent_report']
        print(f"\n{'─' * 80}")
        print("⚠️  DISSENT REPORT")
        print('─' * 80)
        print(f"\n   Majority voted: {report.decision.value.upper()} ({report.majority_count} experts)")
        print(f"   Dissenting: {report.dissent_count} expert(s)")
        
        print(f"\n   Dissenting experts: {', '.join(report.dissent_experts)}")
        
        if report.concerns_raised:
            print(f"\n   Key concerns raised:")
            for concern in report.concerns_raised[:3]:
                wrapped = textwrap.fill(f"• {concern}", width=72,
                                      initial_indent='     ', subsequent_indent='       ')
                print(wrapped)

    # Session info
    print(f"\n{'─' * 80}")
    print(f"Session ID: {result['session_id']}")
    print(f"Review Duration: {sum(r.review_duration_seconds for r in result['reviews']):.1f}s total")
    print("=" * 80 + "\n")

    # Return decision for programmatic use
    return decision.decision == VoteChoice.APPROVE


if __name__ == "__main__":
    try:
        approved = main()
        exit(0 if approved else 1)
    except KeyboardInterrupt:
        print("\n\nReview interrupted by user")
        exit(130)
    except Exception as e:
        print(f"\n\nERROR: {e}")
        import traceback
        traceback.print_exc()
        exit(1)
