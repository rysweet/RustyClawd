#!/usr/bin/env python3
"""Demonstration of Expert Panel Review orchestration pattern.

This demonstrates various use cases for the Expert Panel Review pattern:
1. Basic code review with default experts
2. Custom expert panel for API design
3. Security audit with unanimous requirement
4. Integration with N-Version Programming
"""

import sys
from pathlib import Path

# Add orchestration module to path
sys.path.insert(0, str(Path(__file__).parent / ".claude/tools/amplihack"))

from orchestration.patterns.expert_panel import run_expert_panel, VoteChoice


def demo_basic_code_review():
    """Demo 1: Basic code review with default experts."""
    print("\n" + "=" * 80)
    print("DEMO 1: Basic Code Review (Security, Performance, Simplicity)")
    print("=" * 80 + "\n")

    code_to_review = """
import hashlib
import os

def hash_password(password: str) -> tuple[str, str]:
    '''Hash a password with salt.'''
    salt = os.urandom(32)
    pwdhash = hashlib.pbkdf2_hmac('sha256', 
                                   password.encode('utf-8'), 
                                   salt, 
                                   100000)
    return pwdhash.hex(), salt.hex()

def verify_password(stored_hash: str, stored_salt: str, 
                    password: str) -> bool:
    '''Verify a stored password against one provided by user.'''
    salt = bytes.fromhex(stored_salt)
    pwdhash = hashlib.pbkdf2_hmac('sha256',
                                   password.encode('utf-8'),
                                   salt,
                                   100000)
    return pwdhash.hex() == stored_hash
"""

    result = run_expert_panel(
        solution=code_to_review,
        aggregation_method="simple_majority",
        quorum=3,
        timeout=120
    )

    print_results(result)
    return result


def demo_custom_expert_panel():
    """Demo 2: Custom expert panel for API design review."""
    print("\n" + "=" * 80)
    print("DEMO 2: Custom Expert Panel - API Design Review")
    print("=" * 80 + "\n")

    api_design = """
# User Authentication API Design

## Endpoints

POST /api/v1/auth/register
- Body: { "email": string, "password": string, "name": string }
- Returns: { "user_id": string, "token": string }

POST /api/v1/auth/login
- Body: { "email": string, "password": string }
- Returns: { "token": string, "expires_at": timestamp }

POST /api/v1/auth/logout
- Headers: Authorization: Bearer {token}
- Returns: { "success": boolean }

GET /api/v1/auth/refresh
- Headers: Authorization: Bearer {token}
- Returns: { "token": string, "expires_at": timestamp }

## Security
- Passwords hashed with bcrypt
- JWT tokens with 1-hour expiration
- Refresh tokens stored in HTTP-only cookies
- Rate limiting: 5 requests per minute per IP

## Validation
- Email: RFC 5322 compliant
- Password: Min 8 chars, requires uppercase, lowercase, number
- Name: 2-50 characters, alphanumeric + spaces
"""

    custom_experts = [
        {"domain": "api_design", "focus": "REST API design principles, endpoint structure, HTTP methods"},
        {"domain": "security", "focus": "authentication, authorization, token management, attack vectors"},
        {"domain": "validation", "focus": "input validation, data sanitization, error handling"},
        {"domain": "documentation", "focus": "API clarity, completeness, developer experience"},
    ]

    result = run_expert_panel(
        solution=api_design,
        experts=custom_experts,
        aggregation_method="weighted",  # Weight by confidence
        quorum=3,
        timeout=120
    )

    print_results(result)
    return result


def demo_security_audit():
    """Demo 3: Security audit requiring unanimous approval."""
    print("\n" + "=" * 80)
    print("DEMO 3: Security Audit - Unanimous Requirement")
    print("=" * 80 + "\n")

    security_critical_code = """
# JWT Token Generation for Authentication

import jwt
import datetime
from flask import current_app

def generate_auth_token(user_id: int) -> str:
    '''Generate JWT authentication token.'''
    payload = {
        'user_id': user_id,
        'exp': datetime.datetime.utcnow() + datetime.timedelta(hours=24),
        'iat': datetime.datetime.utcnow()
    }
    
    token = jwt.encode(
        payload,
        current_app.config['SECRET_KEY'],
        algorithm='HS256'
    )
    
    return token

def verify_auth_token(token: str) -> int:
    '''Verify JWT token and return user_id.'''
    try:
        payload = jwt.decode(
            token,
            current_app.config['SECRET_KEY'],
            algorithms=['HS256']
        )
        return payload['user_id']
    except jwt.ExpiredSignatureError:
        return None
    except jwt.InvalidTokenError:
        return None
"""

    security_experts = [
        {"domain": "authentication", "focus": "token security, session management, auth mechanisms"},
        {"domain": "cryptography", "focus": "encryption, key management, algorithm selection"},
        {"domain": "vulnerability", "focus": "common attack vectors, OWASP top 10, exploit prevention"},
    ]

    result = run_expert_panel(
        solution=security_critical_code,
        experts=security_experts,
        aggregation_method="unanimous",  # ALL must approve
        quorum=3,
        timeout=120
    )

    print_results(result)
    return result


def demo_weighted_decision():
    """Demo 4: Weighted voting based on confidence."""
    print("\n" + "=" * 80)
    print("DEMO 4: Weighted Decision Making")
    print("=" * 80 + "\n")

    algorithm_implementation = """
# Binary Search Implementation

def binary_search(arr: list[int], target: int) -> int:
    '''
    Binary search algorithm - finds index of target in sorted array.
    Returns -1 if not found.
    '''
    left, right = 0, len(arr) - 1
    
    while left <= right:
        mid = (left + right) // 2
        
        if arr[mid] == target:
            return mid
        elif arr[mid] < target:
            left = mid + 1
        else:
            right = mid - 1
    
    return -1

# Test cases
assert binary_search([1, 2, 3, 4, 5], 3) == 2
assert binary_search([1, 2, 3, 4, 5], 6) == -1
assert binary_search([1], 1) == 0
assert binary_search([], 1) == -1
"""

    result = run_expert_panel(
        solution=algorithm_implementation,
        aggregation_method="weighted",  # High-confidence votes matter more
        quorum=3,
        timeout=120
    )

    print_results(result)
    return result


def print_results(result: dict):
    """Print formatted results from expert panel review."""
    print(f"\nSession ID: {result['session_id']}")
    print(f"Success: {result['success']}")
    print("-" * 80)

    # Decision summary
    decision = result['decision']
    if decision:
        print(f"\n{'='*40}")
        print(f"FINAL DECISION: {decision.decision.value.upper()}")
        print(f"{'='*40}")
        print(f"Confidence: {decision.confidence:.2f}")
        print(f"Consensus Type: {decision.consensus_type}")
        print(f"Agreement: {decision.agreement_percentage:.1f}%")
        print(f"Quorum Met: {decision.quorum_met}")
        print(f"\nVote Breakdown:")
        print(f"  ✓ Approve: {decision.approve_votes}")
        print(f"  ✗ Reject:  {decision.reject_votes}")
        print(f"  - Abstain: {decision.abstain_votes}")
        print(f"  = Total:   {decision.total_votes}")

    # Individual reviews
    print(f"\n{'='*40}")
    print("INDIVIDUAL EXPERT REVIEWS")
    print(f"{'='*40}")

    for i, review in enumerate(result['reviews'], 1):
        print(f"\n[Expert {i}: {review.domain.upper()}]")
        print(f"Vote: {review.vote.value.upper()} (confidence: {review.confidence:.2f})")
        print(f"Duration: {review.review_duration_seconds:.1f}s")
        
        print(f"\nVote Rationale:")
        print(f"  {review.vote_rationale}")
        
        if review.strengths:
            print(f"\nStrengths:")
            for strength in review.strengths[:3]:  # Show top 3
                print(f"  ✓ {strength}")
        
        if review.weaknesses:
            print(f"\nWeaknesses:")
            for weakness in review.weaknesses[:3]:  # Show top 3
                print(f"  ✗ {weakness}")
        
        if review.domain_scores:
            print(f"\nDomain Scores:")
            for aspect, score in list(review.domain_scores.items())[:3]:
                bar = "█" * int(score * 10)
                print(f"  {aspect}: {score:.2f} {bar}")
        
        print("-" * 40)

    # Dissent report
    if result['dissent_report']:
        report = result['dissent_report']
        print(f"\n{'='*40}")
        print("DISSENT REPORT")
        print(f"{'='*40}")
        print(f"Majority: {report.majority_count} experts voted {report.decision.value.upper()}")
        print(f"Dissent: {report.dissent_count} experts dissented")
        
        print(f"\nDissenting Experts: {', '.join(report.dissent_experts)}")
        
        print(f"\nDissenting Rationales:")
        for expert, rationale in zip(report.dissent_experts, report.dissent_rationales):
            print(f"  [{expert}]: {rationale[:200]}...")
        
        if report.concerns_raised:
            print(f"\nKey Concerns Raised:")
            for concern in report.concerns_raised[:5]:
                print(f"  ⚠ {concern}")

    print(f"\n{'='*80}\n")


def main():
    """Run all demonstrations."""
    print("\n" + "=" * 80)
    print("EXPERT PANEL REVIEW PATTERN - COMPREHENSIVE DEMONSTRATION")
    print("=" * 80)
    print("\nThis demonstration shows the Expert Panel Review orchestration pattern")
    print("where multiple expert agents independently review solutions and vote.")
    print("\nFeatures demonstrated:")
    print("  • Default expert panel (security, performance, simplicity)")
    print("  • Custom expert panels for specific domains")
    print("  • Multiple aggregation methods (simple majority, weighted, unanimous)")
    print("  • Quorum requirements and dissent reporting")
    print("  • Byzantine-robust decision making")

    demos = [
        ("Basic Code Review", demo_basic_code_review),
        ("Custom Expert Panel", demo_custom_expert_panel),
        ("Security Audit", demo_security_audit),
        ("Weighted Decision", demo_weighted_decision),
    ]

    results = []
    for name, demo_func in demos:
        try:
            print(f"\n\nStarting: {name}")
            result = demo_func()
            results.append((name, result, True))
        except KeyboardInterrupt:
            print("\n\nDemo interrupted by user")
            break
        except Exception as e:
            print(f"\n\nERROR in {name}: {e}")
            results.append((name, None, False))

    # Summary
    print("\n" + "=" * 80)
    print("DEMONSTRATION SUMMARY")
    print("=" * 80)
    
    for name, result, success in results:
        status = "✓ SUCCESS" if success else "✗ FAILED"
        print(f"\n{status}: {name}")
        
        if result and result.get('decision'):
            decision = result['decision']
            print(f"  Decision: {decision.decision.value.upper()}")
            print(f"  Consensus: {decision.consensus_type}")
            print(f"  Votes: {decision.approve_votes}A / {decision.reject_votes}R / {decision.abstain_votes}Ab")
            print(f"  Confidence: {decision.confidence:.2f}")

    print("\n" + "=" * 80)
    print("Demonstrations complete!")
    print("=" * 80 + "\n")


if __name__ == "__main__":
    main()
