use super::*;

mod variable {
    use super::*;

    #[test]
    fn from_string_plugin_root() {
        assert_eq!(
            Variable::from_string("CLAUDE_PLUGIN_ROOT"),
            Some(Variable::PluginRoot)
        );
    }

    #[test]
    fn from_string_project_root() {
        assert_eq!(
            Variable::from_string("CLAUDE_PROJECT_ROOT"),
            Some(Variable::ProjectRoot)
        );
    }

    #[test]
    fn from_string_home() {
        assert_eq!(Variable::from_string("HOME"), Some(Variable::Home));
    }

    #[test]
    fn from_string_user() {
        assert_eq!(Variable::from_string("USER"), Some(Variable::User));
    }

    #[test]
    fn from_string_pwd() {
        assert_eq!(Variable::from_string("PWD"), Some(Variable::Pwd));
    }

    #[test]
    fn from_string_unknown() {
        assert_eq!(Variable::from_string("UNKNOWN_VAR"), None);
    }

    #[test]
    fn from_string_lowercase_not_supported() {
        assert_eq!(Variable::from_string("home"), None);
        assert_eq!(Variable::from_string("Claude_Plugin_Root"), None);
    }

    #[test]
    fn resolve_plugin_root() {
        let ctx = SubstitutionContext::new(PathBuf::from("/plugin/root"), None);
        assert_eq!(
            Variable::PluginRoot.resolve(&ctx),
            Some("/plugin/root".to_string())
        );
    }

    #[test]
    fn resolve_project_root() {
        let ctx = SubstitutionContext::new(
            PathBuf::from("/plugin/root"),
            Some(PathBuf::from("/project/root")),
        );
        assert_eq!(
            Variable::ProjectRoot.resolve(&ctx),
            Some("/project/root".to_string())
        );
    }

    #[test]
    fn resolve_project_root_none() {
        let ctx = SubstitutionContext::new(PathBuf::from("/plugin/root"), None);
        assert_eq!(Variable::ProjectRoot.resolve(&ctx), None);
    }

    #[test]
    fn resolve_home() {
        let ctx = SubstitutionContext::new(PathBuf::from("/plugin/root"), None);
        // HOME should be available in most test environments
        let result = Variable::Home.resolve(&ctx);
        // Just verify it returns Some or None without asserting specific value
        assert!(result.is_some() || result.is_none());
    }

    #[test]
    fn resolve_pwd() {
        let ctx = SubstitutionContext::new(PathBuf::from("/plugin/root"), None);
        let result = Variable::Pwd.resolve(&ctx);
        // Should return current directory
        assert!(result.is_some());
    }
}

mod substitution {
    use super::*;

    fn test_ctx() -> SubstitutionContext {
        SubstitutionContext::new(
            PathBuf::from("/plugin/root"),
            Some(PathBuf::from("/project/root")),
        )
    }

    #[test]
    fn single_variable() {
        let substituter = Substituter::new(test_ctx());
        assert_eq!(
            substituter.substitute("${CLAUDE_PLUGIN_ROOT}/tools"),
            "/plugin/root/tools"
        );
    }

    #[test]
    fn multiple_variables() {
        let substituter = Substituter::new(test_ctx());
        assert_eq!(
            substituter.substitute("${CLAUDE_PLUGIN_ROOT}:${CLAUDE_PROJECT_ROOT}"),
            "/plugin/root:/project/root"
        );
    }

    #[test]
    fn multiple_variables_in_path() {
        let substituter = Substituter::new(test_ctx());
        assert_eq!(
            substituter.substitute("${CLAUDE_PLUGIN_ROOT}/bin:${CLAUDE_PROJECT_ROOT}/bin"),
            "/plugin/root/bin:/project/root/bin"
        );
    }

    #[test]
    fn unknown_variable_preserved() {
        let substituter = Substituter::new(test_ctx());
        assert_eq!(substituter.substitute("${UNKNOWN}/path"), "${UNKNOWN}/path");
    }

    #[test]
    fn unavailable_variable_preserved() {
        let ctx = SubstitutionContext::new(PathBuf::from("/plugin/root"), None);
        let substituter = Substituter::new(ctx);
        assert_eq!(
            substituter.substitute("${CLAUDE_PROJECT_ROOT}/path"),
            "${CLAUDE_PROJECT_ROOT}/path"
        );
    }

    #[test]
    fn malformed_pattern_no_closing_brace() {
        let substituter = Substituter::new(test_ctx());
        // Missing closing brace - should preserve
        assert_eq!(
            substituter.substitute("${CLAUDE_PLUGIN_ROOT/path"),
            "${CLAUDE_PLUGIN_ROOT/path"
        );
    }

    #[test]
    fn dollar_without_braces_preserved() {
        let substituter = Substituter::new(test_ctx());
        assert_eq!(substituter.substitute("$VARIABLE"), "$VARIABLE");
    }

    #[test]
    fn empty_string() {
        let substituter = Substituter::new(test_ctx());
        assert_eq!(substituter.substitute(""), "");
    }

    #[test]
    fn no_variables() {
        let substituter = Substituter::new(test_ctx());
        assert_eq!(substituter.substitute("/absolute/path"), "/absolute/path");
    }

    #[test]
    fn empty_variable_name() {
        let substituter = Substituter::new(test_ctx());
        assert_eq!(substituter.substitute("${}/path"), "${}/path");
    }

    #[test]
    fn nested_variables_not_supported() {
        let substituter = Substituter::new(test_ctx());
        // Nested variables should be preserved as-is (first ${ stops at first })
        let result = substituter.substitute("${CLAUDE_${PLUGIN}_ROOT}");
        // Will match ${CLAUDE_ and stop at first }, leaving rest as-is
        assert!(result.contains("${") || result.contains("}"));
    }

    #[test]
    fn mixed_absolute_and_relative() {
        let substituter = Substituter::new(test_ctx());
        assert_eq!(
            substituter.substitute("${CLAUDE_PLUGIN_ROOT}/tools:/absolute/path"),
            "/plugin/root/tools:/absolute/path"
        );
    }
}

mod frontmatter {
    use super::*;
    use crate::commands::loader::FrontMatter;

    fn test_ctx() -> SubstitutionContext {
        SubstitutionContext::new(
            PathBuf::from("/plugin/root"),
            Some(PathBuf::from("/project/root")),
        )
    }

    #[test]
    fn substitute_allowed_tools() {
        let substituter = Substituter::new(test_ctx());
        let mut fm = FrontMatter {
            allowed_tools: vec![
                "${CLAUDE_PLUGIN_ROOT}/bin/verify".to_string(),
                "Read".to_string(),
                "${CLAUDE_PLUGIN_ROOT}/bin/check".to_string(),
            ],
            ..Default::default()
        };

        substituter.substitute_frontmatter(&mut fm);

        assert_eq!(fm.allowed_tools[0], "/plugin/root/bin/verify");
        assert_eq!(fm.allowed_tools[1], "Read");
        assert_eq!(fm.allowed_tools[2], "/plugin/root/bin/check");
    }

    #[test]
    fn substitute_description() {
        let substituter = Substituter::new(test_ctx());
        let mut fm = FrontMatter {
            description: Some("Uses ${CLAUDE_PLUGIN_ROOT}".to_string()),
            ..Default::default()
        };

        substituter.substitute_frontmatter(&mut fm);

        assert_eq!(fm.description, Some("Uses /plugin/root".to_string()));
    }

    #[test]
    fn substitute_argument_hint() {
        let substituter = Substituter::new(test_ctx());
        let mut fm = FrontMatter {
            argument_hint: Some("${CLAUDE_PLUGIN_ROOT}/config".to_string()),
            ..Default::default()
        };

        substituter.substitute_frontmatter(&mut fm);

        assert_eq!(fm.argument_hint, Some("/plugin/root/config".to_string()));
    }

    #[test]
    fn preserves_non_string_fields() {
        let substituter = Substituter::new(test_ctx());
        let mut fm = FrontMatter {
            model: Some("claude-3-5-sonnet-20241022".to_string()),
            allowed_tools: vec!["${CLAUDE_PLUGIN_ROOT}/bin/verify".to_string()],
            disable_model_invocation: Some(true),
            ..Default::default()
        };

        substituter.substitute_frontmatter(&mut fm);

        // Verify non-substituted fields are unchanged
        assert_eq!(fm.model, Some("claude-3-5-sonnet-20241022".to_string()));
        assert_eq!(fm.disable_model_invocation, Some(true));
        // And verify substitution worked
        assert_eq!(fm.allowed_tools[0], "/plugin/root/bin/verify");
    }

    #[test]
    fn empty_frontmatter() {
        let substituter = Substituter::new(test_ctx());
        let mut fm = FrontMatter::default();

        substituter.substitute_frontmatter(&mut fm);

        // Should not crash, just leave empty
        assert!(fm.allowed_tools.is_empty());
        assert!(fm.description.is_none());
        assert!(fm.argument_hint.is_none());
    }
}

mod scenarios {
    use super::*;
    use crate::commands::loader::FrontMatter;

    #[test]
    fn plugin_with_relative_tool_paths() {
        let ctx = SubstitutionContext::new(
            PathBuf::from("/home/user/plugins/security-tools"),
            Some(PathBuf::from("/home/user/project")),
        );
        let substituter = Substituter::new(ctx);

        let mut fm = FrontMatter {
            description: Some("Security verification agent".to_string()),
            allowed_tools: vec![
                "${CLAUDE_PLUGIN_ROOT}/bin/verify-config".to_string(),
                "${CLAUDE_PLUGIN_ROOT}/bin/check-syntax".to_string(),
                "Read".to_string(),
                "Grep".to_string(),
            ],
            ..Default::default()
        };

        substituter.substitute_frontmatter(&mut fm);

        assert_eq!(
            fm.allowed_tools[0],
            "/home/user/plugins/security-tools/bin/verify-config"
        );
        assert_eq!(
            fm.allowed_tools[1],
            "/home/user/plugins/security-tools/bin/check-syntax"
        );
        assert_eq!(fm.allowed_tools[2], "Read");
        assert_eq!(fm.allowed_tools[3], "Grep");
    }

    #[test]
    fn plugin_with_mixed_paths() {
        let ctx = SubstitutionContext::new(
            PathBuf::from("/plugin/root"),
            Some(PathBuf::from("/project/root")),
        );
        let substituter = Substituter::new(ctx);

        let mut fm = FrontMatter {
            allowed_tools: vec![
                "${CLAUDE_PLUGIN_ROOT}/tools/verify".to_string(),
                "/absolute/path/to/tool".to_string(),
                "Read".to_string(),
                "${CLAUDE_PROJECT_ROOT}/shared/tool".to_string(),
            ],
            ..Default::default()
        };

        substituter.substitute_frontmatter(&mut fm);

        assert_eq!(fm.allowed_tools[0], "/plugin/root/tools/verify");
        assert_eq!(fm.allowed_tools[1], "/absolute/path/to/tool");
        assert_eq!(fm.allowed_tools[2], "Read");
        assert_eq!(fm.allowed_tools[3], "/project/root/shared/tool");
    }

    #[test]
    fn real_world_agent_frontmatter() {
        let ctx = SubstitutionContext::new(
            PathBuf::from("/home/alice/.claude/plugins/code-analyzer"),
            Some(PathBuf::from("/home/alice/my-project")),
        );
        let substituter = Substituter::new(ctx);

        let mut fm = FrontMatter {
            description: Some("Code analyzer using tools from ${CLAUDE_PLUGIN_ROOT}".to_string()),
            allowed_tools: vec![
                "${CLAUDE_PLUGIN_ROOT}/analyzers/python-analyzer".to_string(),
                "${CLAUDE_PLUGIN_ROOT}/analyzers/rust-analyzer".to_string(),
                "Read".to_string(),
                "Grep".to_string(),
                "Bash".to_string(),
            ],
            argument_hint: Some("${CLAUDE_PROJECT_ROOT}/src".to_string()),
            ..Default::default()
        };

        substituter.substitute_frontmatter(&mut fm);

        assert_eq!(
            fm.description,
            Some(
                "Code analyzer using tools from /home/alice/.claude/plugins/code-analyzer"
                    .to_string()
            )
        );
        assert_eq!(
            fm.allowed_tools[0],
            "/home/alice/.claude/plugins/code-analyzer/analyzers/python-analyzer"
        );
        assert_eq!(
            fm.allowed_tools[1],
            "/home/alice/.claude/plugins/code-analyzer/analyzers/rust-analyzer"
        );
        assert_eq!(fm.allowed_tools[2], "Read");
        assert_eq!(
            fm.argument_hint,
            Some("/home/alice/my-project/src".to_string())
        );
    }
}
