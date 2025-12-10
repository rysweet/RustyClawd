#!/usr/bin/env python3
"""Expert Panel Review Pattern - Interactive Demonstration

This script demonstrates the Expert Panel Review orchestration pattern with
multiple examples showing different configurations and use cases.
"""

import sys
from pathlib import Path

# Add orchestration patterns to path
sys.path.insert(0, str(Path(__file__).parent / ".claude/tools/amplihack/orchestration"))

from patterns import run_expert_panel


def print_section_header(title: str):
    """Print a formatted section header."""
    print("\n" + "=" * 80)
    print(f"  {title}")
    print("=" * 80 + "\n")


def print_result(result: dict):
    """Print formatted expert panel result."""
    decision = result["decision"]
    
    print(f"Session ID: {result['session_id']}")
    print(f"Success: {result['success']}")
    print(f"\n--- FINAL DECISION ---")
    print(f"Decision: {decision.decision.value.upper()}")
    print(f"Confidence: {decision.confidence:.2f}")
    print(f"Consensus Type: {decision.consensus_type}")
    print(f"Agreement: {decision.agreement_percentage:.1f}%")
    print(f"Quorum Met: {decision.quorum_met}")
    
    print(f"\n--- VOTE BREAKDOWN ---")
    print(f"Approve: {decision.approve_votes}")
    print(f"Reject:  {decision.reject_votes}")
    print(f"Abstain: {decision.abstain_votes}")
    print(f"Total:   {decision.total_votes}")
    
    print(f"\n--- EXPERT REVIEWS ---")
    for review in result["reviews"]:
        print(f"\n{review.domain.upper()} Expert:")
        print(f"  Vote: {review.vote.value.upper()} (confidence: {review.confidence:.2f})")
        print(f"  Rationale: {review.vote_rationale[:200]}...")
        
        if review.strengths:
            print(f"  Strengths: {len(review.strengths)} identified")
            for strength in review.strengths[:2]:
                print(f"    - {strength[:80]}...")
        
        if review.weaknesses:
            print(f"  Weaknesses: {len(review.weaknesses)} identified")
            for weakness in review.weaknesses[:2]:
                print(f"    - {weakness[:80]}...")
    
    if result["dissent_report"]:
        print(f"\n--- DISSENT REPORT ---")
        report = result["dissent_report"]
        print(f"Decision: {report.decision.value.upper()} ({report.majority_count} votes)")
        print(f"Dissenting: {report.dissent_count} votes")
        print(f"Dissenting Experts: {', '.join(report.dissent_experts)}")
        print(f"\nDissenting Rationales:")
        for expert, rationale in zip(report.dissent_experts, report.dissent_rationales):
            print(f"  {expert}: {rationale[:150]}...")
        
        if report.concerns_raised:
            print(f"\nKey Concerns ({len(report.concerns_raised)}):")
            for concern in report.concerns_raised[:3]:
                print(f"  - {concern[:100]}...")


def example_1_code_review():
    """Example 1: Simple code review with default experts."""
    print_section_header("EXAMPLE 1: Code Review - Simple Majority")
    
    code_to_review = """
def authenticate_user(username, password):
    '''Authenticate user with username and password.'''
    # Connect to database
    conn = sqlite3.connect('users.db')
    cursor = conn.cursor()
    
    # Query user
    query = f"SELECT * FROM users WHERE username='{username}' AND password='{password}'"
    cursor.execute(query)
    user = cursor.fetchone()
    
    conn.close()
    
    if user:
        # Store user ID in session
        session['user_id'] = user[0]
        return True
    return False
"""
    
    print("Reviewing code with SQL injection vulnerability...")
    print(f"Code snippet:\n{code_to_review}")
    
    result = run_expert_panel(
        solution=code_to_review,
        aggregation_method="simple_majority",
        quorum=3
    )
    
    print_result(result)
    return result


def example_2_security_audit():
    """Example 2: Security audit with unanimous requirement."""
    print_section_header("EXAMPLE 2: Security Audit - Unanimous Approval Required")
    
    secure_code = """
import bcrypt
from typing import Tuple

def hash_password(password: str, rounds: int = 12) -> Tuple[bytes, bytes]:
    '''Hash a password using bcrypt with configurable work factor.
    
    Args:
        password: Plain text password to hash
        rounds: Number of bcrypt rounds (default: 12)
        
    Returns:
        Tuple of (hashed_password, salt)
    '''
    if not password or len(password) < 8:
        raise ValueError("Password must be at least 8 characters")
    
    if rounds < 10 or rounds > 16:
        raise ValueError("Rounds must be between 10 and 16")
    
    # Generate salt
    salt = bcrypt.gensalt(rounds=rounds)
    
    # Hash password
    hashed = bcrypt.hashpw(password.encode('utf-8'), salt)
    
    return hashed, salt


def verify_password(password: str, hashed: bytes) -> bool:
    '''Verify a password against a bcrypt hash.
    
    Args:
        password: Plain text password to verify
        hashed: Previously hashed password
        
    Returns:
        True if password matches, False otherwise
    '''
    try:
        return bcrypt.checkpw(password.encode('utf-8'), hashed)
    except Exception:
        return False
"""
    
    print("Reviewing security-critical password hashing implementation...")
    print(f"Code snippet (first 20 lines):\n{chr(10).join(secure_code.split(chr(10))[:20])}\n...")
    
    # Custom security-focused experts
    security_experts = [
        {"domain": "authentication", "focus": "password storage, hashing algorithms, credential security"},
        {"domain": "cryptography", "focus": "encryption strength, key derivation, randomness"},
        {"domain": "input_validation", "focus": "input sanitization, boundary checking, error handling"},
    ]
    
    result = run_expert_panel(
        solution=secure_code,
        experts=security_experts,
        aggregation_method="unanimous",  # ALL must approve
        quorum=3
    )
    
    print_result(result)
    return result


def example_3_api_design():
    """Example 3: API design review with weighted voting."""
    print_section_header("EXAMPLE 3: API Design Review - Weighted by Confidence")
    
    api_design = """
REST API Design for User Service

# Endpoints

## GET /api/users
- List all users
- Query params: ?page=1&limit=20&sort=created_at
- Response: { users: [...], total: N, page: X }

## GET /api/users/:id
- Get single user by ID
- Response: { user: {...} }

## POST /api/users
- Create new user
- Body: { username, email, password }
- Response: { user: {...}, token: "..." }

## PUT /api/users/:id
- Update user (full replacement)
- Body: { username, email, password }
- Response: { user: {...} }

## PATCH /api/users/:id
- Partial user update
- Body: { field: value, ... }
- Response: { user: {...} }

## DELETE /api/users/:id
- Delete user
- Response: { success: true }

# Authentication
- JWT tokens in Authorization header
- Tokens expire after 24 hours
- No refresh token mechanism

# Rate Limiting
- 100 requests per minute per IP
- Returns 429 when exceeded

# Error Handling
- Standard HTTP status codes
- Error format: { error: "message" }
"""
    
    print("Reviewing RESTful API design...")
    print(f"Design document (first 20 lines):\n{chr(10).join(api_design.split(chr(10))[:20])}\n...")
    
    # Custom API design experts
    api_experts = [
        {"domain": "api_design", "focus": "REST principles, endpoint structure, versioning"},
        {"domain": "security", "focus": "authentication, authorization, data protection"},
        {"domain": "scalability", "focus": "performance, caching, rate limiting"},
        {"domain": "developer_experience", "focus": "API usability, documentation, consistency"},
    ]
    
    result = run_expert_panel(
        solution=api_design,
        experts=api_experts,
        aggregation_method="weighted",  # Weight by confidence
        quorum=3
    )
    
    print_result(result)
    return result


def example_4_architecture_review():
    """Example 4: Architecture review with default experts."""
    print_section_header("EXAMPLE 4: System Architecture Review")
    
    architecture = """
E-Commerce Platform Architecture

## Components

### Frontend
- React SPA hosted on CloudFront CDN
- Client-side routing
- State management: Redux
- Real-time updates via WebSockets

### Backend Services
1. API Gateway (Node.js + Express)
   - Request routing
   - Authentication/authorization
   - Rate limiting
   
2. Product Service (Python + FastAPI)
   - Product catalog
   - Search (Elasticsearch)
   - Image processing
   
3. Order Service (Java + Spring Boot)
   - Order management
   - Payment processing (Stripe integration)
   - Order history
   
4. User Service (Go)
   - User authentication (JWT)
   - Profile management
   - Session management (Redis)

### Data Layer
- PostgreSQL (primary database)
  - Users, products, orders
  - Full ACID compliance
  
- Redis (caching + sessions)
  - Session storage
  - Product catalog cache
  - Real-time inventory
  
- S3 (object storage)
  - Product images
  - Invoice PDFs

### Infrastructure
- Kubernetes for orchestration
- All services in same cluster
- Single region deployment (us-east-1)
- Horizontal pod autoscaling
- No disaster recovery plan

### Monitoring
- Prometheus + Grafana
- Application logs to CloudWatch
- No distributed tracing
"""
    
    print("Reviewing microservices architecture for e-commerce platform...")
    print(f"Architecture document (first 25 lines):\n{chr(10).join(architecture.split(chr(10))[:25])}\n...")
    
    result = run_expert_panel(
        solution=architecture,
        aggregation_method="simple_majority",
        quorum=3
    )
    
    print_result(result)
    return result


def example_5_combined_with_n_version():
    """Example 5: Combine Expert Panel with N-Version Programming."""
    print_section_header("EXAMPLE 5: Expert Panel + N-Version Integration")
    
    print("This example shows how to combine Expert Panel with N-Version Programming:")
    print("1. N-Version generates multiple implementations")
    print("2. Expert Panel reviews each implementation")
    print("3. Select implementation with strongest approval")
    
    print("\nCode structure for integration:")
    print("""
from patterns import run_n_version, run_expert_panel

# Step 1: Generate 3 implementations
n_version_result = run_n_version(
    task_prompt="Implement JWT token validation",
    n=3
)

# Step 2: Expert panel reviews each implementation
panel_results = []
for i, version_result in enumerate(n_version_result["versions"]):
    if version_result.exit_code == 0:
        panel_result = run_expert_panel(
            solution=version_result.output,
            aggregation_method="simple_majority",
            quorum=3
        )
        panel_results.append({
            "version": i + 1,
            "decision": panel_result["decision"],
            "confidence": panel_result["decision"].confidence
        })

# Step 3: Select version with strongest approval
best_version = max(
    panel_results,
    key=lambda x: (
        x["decision"].approve_votes,
        x["confidence"]
    )
)

print(f"Selected version {best_version['version']}")
print(f"  Approve votes: {best_version['decision'].approve_votes}")
print(f"  Confidence: {best_version['confidence']:.2f}")
""")
    
    print("\n[Demonstration would execute actual N-Version + Expert Panel here]")


def main():
    """Run expert panel demonstration."""
    print("\n" + "#" * 80)
    print("#  Expert Panel Review Orchestration Pattern - Interactive Demo")
    print("#" * 80)
    
    print("""
The Expert Panel Review pattern provides Byzantine-robust decision-making through:
  • Parallel independent expert reviews
  • Vote-based decisions (APPROVE/REJECT/ABSTAIN)
  • Multiple aggregation methods (simple majority, weighted, unanimous)
  • Dissent reporting for transparency

This demo includes 5 examples showing different use cases and configurations.
""")
    
    # Prompt user for which examples to run
    print("\nSelect examples to run:")
    print("  1. Code Review (SQL injection vulnerability)")
    print("  2. Security Audit (password hashing - unanimous)")
    print("  3. API Design Review (weighted voting)")
    print("  4. Architecture Review (microservices)")
    print("  5. Integration Pattern (Expert Panel + N-Version)")
    print("  0. Run all examples")
    
    try:
        choice = input("\nEnter choice (0-5): ").strip()
    except (KeyboardInterrupt, EOFError):
        print("\n\nExiting...")
        return
    
    examples = {
        "1": example_1_code_review,
        "2": example_2_security_audit,
        "3": example_3_api_design,
        "4": example_4_architecture_review,
        "5": example_5_combined_with_n_version,
    }
    
    if choice == "0":
        # Run all examples
        for example_func in examples.values():
            try:
                example_func()
                input("\nPress Enter to continue to next example...")
            except KeyboardInterrupt:
                print("\n\nSkipping to next example...")
                continue
    elif choice in examples:
        examples[choice]()
    else:
        print(f"\nInvalid choice: {choice}")
        return
    
    print("\n" + "#" * 80)
    print("#  Expert Panel Review Demo Complete")
    print("#" * 80 + "\n")
    
    print("Key Takeaways:")
    print("  • Default experts: security, performance, simplicity")
    print("  • Custom experts: Define domain-specific review panels")
    print("  • Aggregation methods: simple_majority, weighted, unanimous")
    print("  • Quorum requirement: Ensures enough non-abstain votes")
    print("  • Dissent reports: Minority opinions preserved")
    print("\nFor more details, see: .claude/commands/amplihack/expert-panel.md")


if __name__ == "__main__":
    main()
