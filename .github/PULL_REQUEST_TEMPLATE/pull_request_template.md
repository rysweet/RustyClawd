## Description

<!-- Provide a clear and concise description of your changes -->

Closes #<!-- issue number -->

## Type of Change

<!-- Mark the relevant option with an 'x' -->

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Refactoring (no functional changes)
- [ ] Performance improvement
- [ ] Test additions or improvements

## Testing

<!-- Describe the testing you've done -->

- [ ] All existing tests pass (`cargo test`)
- [ ] New tests added for this change
- [ ] Manual testing performed (describe below)

**Manual Testing Details:**
<!-- Describe what you tested manually -->

## Code Quality Checklist

- [ ] Code follows project style guidelines (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] No compiler warnings
- [ ] No `unwrap()` or `expect()` in production code (use `?` operator)
- [ ] Error handling is appropriate and informative
- [ ] Documentation updated (rustdoc, ARCHITECTURE.md, etc.)

## Branding Checklist

- [ ] **No Claude branding in UI strings** (checked with `./scripts/check-branding.sh`)
- [ ] Branding validation tests pass (`cargo test --test branding_test`)
- [ ] Used "RustyClawd" or "Assistant" instead of "Claude" in user-facing strings
- [ ] N/A - No user-facing strings modified

**Allowed "Claude" contexts (if applicable):**
- [ ] API model names only (e.g., `"claude-sonnet-4-5"`)
- [ ] Internal logging/debugging only
- [ ] Comments/documentation for attribution
- [ ] N/A - No "Claude" references

## Philosophy Compliance

- [ ] Changes follow "Ruthless Simplicity" principle
- [ ] No TODOs, stubs, or unimplemented functions
- [ ] Each module/function has single, clear responsibility
- [ ] Complexity justified by proportional value
- [ ] See [PHILOSOPHY.md](.claude/context/PHILOSOPHY.md) for details

## Screenshots (if applicable)

<!-- Add screenshots for UI changes -->

## Additional Notes

<!-- Any additional context, concerns, or questions -->

## Reviewer Notes

<!-- For reviewers: any specific areas that need extra attention? -->

---

**Before submitting:**
1. Run `./scripts/check-branding.sh` ✅
2. Run `cargo test` ✅
3. Run `cargo clippy -- -D warnings` ✅
4. Run `cargo fmt` ✅
5. Update documentation ✅
