//! Pre-approved domains for WebFetch tool
//!
//! Domains in this list don't require user permission prompts.
//! Organized by ecosystem for easy maintenance.

/// Pre-approved domains that don't require permission prompts
pub const PRE_APPROVED_DOMAINS: &[&str] = &[
    // Anthropic
    "docs.anthropic.com",
    "www.anthropic.com",
    "claude.ai",
    // Python ecosystem
    "docs.python.org",
    "pypi.org",
    "packaging.python.org",
    "peps.python.org",
    "realpython.com",
    // JavaScript/TypeScript ecosystem
    "developer.mozilla.org",
    "nodejs.org",
    "www.typescriptlang.org",
    "npmjs.com",
    "yarnpkg.com",
    // Rust ecosystem
    "doc.rust-lang.org",
    "docs.rs",
    "crates.io",
    "rust-lang.org",
    // Web standards
    "www.w3.org",
    "html.spec.whatwg.org",
    "tc39.es",
    // Cloud providers
    "docs.aws.amazon.com",
    "cloud.google.com",
    "azure.microsoft.com",
    "docs.microsoft.com",
    // Databases
    "dev.mysql.com",
    "www.postgresql.org",
    "docs.mongodb.com",
    "redis.io",
    "cassandra.apache.org",
    // Frameworks
    "reactjs.org",
    "react.dev",
    "vuejs.org",
    "angular.io",
    "svelte.dev",
    "nextjs.org",
    "django-doc-en.readthedocs.io",
    "flask.palletsprojects.com",
    "fastapi.tiangolo.com",
    "spring.io",
    "rubyonrails.org",
    // Tools & platforms
    "git-scm.com",
    "github.com",
    "gitlab.com",
    "stackoverflow.com",
    "en.wikipedia.org",
    "www.reddit.com",
    "dev.to",
    "medium.com",
    // Package registries
    "packagist.org",
    "rubygems.org",
    "nuget.org",
    "mvnrepository.com",
    // Documentation platforms
    "readthedocs.io",
    "readthedocs.org",
    "mkdocs.org",
    "docusaurus.io",
    // Testing
    "jestjs.io",
    "vitest.dev",
    "pytest.org",
    "junit.org",
    // DevOps
    "docs.docker.com",
    "kubernetes.io",
    "www.jenkins.io",
    "circleci.com",
    "docs.gitlab.com",
    // Security
    "owasp.org",
    "cwe.mitre.org",
    "nvd.nist.gov",
    // Standards & RFCs
    "www.ietf.org",
    "datatracker.ietf.org",
    "www.iso.org",
    // Developer resources
    "en.cppreference.com",
    "devdocs.io",
    "learn.microsoft.com",
    "developers.google.com",
    "developer.apple.com",
    // Additional platforms
    "docs.github.com",
    "support.google.com",
    "www.php.net",
    "go.dev",
];
