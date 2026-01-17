# Claude Code Deminification Workflow Diagram

## Overview Flowchart

```mermaid
flowchart TD
    Start([User wants to learn from Claude Code]) --> Check{Have deminified files?}

    Check -->|No| Setup[Run analyze-claude-code.sh]
    Check -->|Yes| Search[Search for patterns]

    Setup --> Install[Install prettier & js-beautify]
    Install --> Find[Find Claude Code installation]
    Find --> Copy[Copy cli.js to research dir]
    Copy --> Deminify[Deminify with prettier & js-beautify]
    Deminify --> Index[Create search indices]
    Index --> Ready[Research directory ready]

    Ready --> Search

    Search --> Method{Search method?}

    Method -->|Quick| Script[Use search-patterns.sh]
    Method -->|Manual| Grep[Direct grep search]
    Method -->|Browse| IDE[Open in VS Code]

    Script --> Results[Pattern results]
    Grep --> Results
    IDE --> Results

    Results --> Analyze[Analyze JavaScript code]
    Analyze --> Translate[Translate to Rust using guide]
    Translate --> Implement[Implement in RustyClawd]
    Implement --> Test[Test for parity]

    Test --> Document[Document findings]
    Document --> End([Done])
```

## Component Architecture

```mermaid
flowchart LR
    subgraph Claude Code
        CLI[cli.js<br/>11MB minified]
    end

    subgraph Tools
        P[prettier]
        JB[js-beautify]
    end

    subgraph Research Directory
        PM[claude-code-minified.js<br/>489k lines]
        JM[claude-code-jsbeautify.js<br/>465k lines]

        subgraph Indices
            IC[index-contentblock.txt]
            IS[index-streaming.txt]
            IH[index-hooks.txt]
            IT[index-tools.txt]
            ISE[index-session.txt]
            ITH[index-thinking.txt]
        end

        F[findings.md]
    end

    subgraph Scripts
        AS[analyze-claude-code.sh]
        SP[search-patterns.sh]
    end

    subgraph Documentation
        DG[DEMINIFICATION_GUIDE.md]
        QS[QUICK_START.md]
        RD[research/README.md]
    end

    CLI -->|Copy| P
    CLI -->|Copy| JB
    P --> PM
    JB --> JM
    PM --> IC
    PM --> IS
    PM --> IH
    PM --> IT
    PM --> ISE
    PM --> ITH

    AS -->|Orchestrates| P
    AS -->|Orchestrates| JB
    SP -->|Searches| PM
    SP -->|Searches| IC

    DG -->|References| AS
    QS -->|Uses| AS
    RD -->|Guides| F
```

## Search Pattern Flow

```mermaid
flowchart TD
    User[User needs to find pattern] --> Question{What pattern?}

    Question -->|ContentBlocks| CB[search-patterns.sh contentblocks]
    Question -->|Streaming| ST[search-patterns.sh streaming]
    Question -->|Hooks| HK[search-patterns.sh hooks]
    Question -->|Tools| TL[search-patterns.sh tools]
    Question -->|Session| SE[search-patterns.sh session]
    Question -->|Thinking| TH[search-patterns.sh thinking]
    Question -->|Custom| CU[search-patterns.sh custom pattern]

    CB --> Results[Results with line numbers]
    ST --> Results
    HK --> Results
    TL --> Results
    SE --> Results
    TH --> Results
    CU --> Results

    Results --> Extract{Need context?}

    Extract -->|Yes| Context[grep -B 10 -A 10 line_number]
    Extract -->|No| Translate

    Context --> Translate[Use JS→Rust guide]
    Translate --> Rust[Implement in Rust]
    Rust --> Verify[Test parity]
    Verify --> Done([Complete])
```

## JavaScript to Rust Translation Pipeline

```mermaid
flowchart LR
    subgraph JavaScript
        JS_Promise[Promises]
        JS_Class[Classes]
        JS_Interface[Interfaces]
        JS_Union[Union Types]
        JS_Callback[Callbacks]
    end

    subgraph Translation Guide
        TG[DEMINIFICATION_GUIDE.md<br/>Pattern Mappings]
    end

    subgraph Rust
        RS_Async[async/await]
        RS_Struct[struct + impl]
        RS_Trait[Traits]
        RS_Enum[Enums with serde]
        RS_Closure[Closures]
    end

    JS_Promise -->|TG| RS_Async
    JS_Class -->|TG| RS_Struct
    JS_Interface -->|TG| RS_Trait
    JS_Union -->|TG| RS_Enum
    JS_Callback -->|TG| RS_Closure

    RS_Async --> Test[Test Implementation]
    RS_Struct --> Test
    RS_Trait --> Test
    RS_Enum --> Test
    RS_Closure --> Test

    Test --> Parity{Matches Claude Code?}
    Parity -->|Yes| Done([Deploy])
    Parity -->|No| Debug[Debug & Refine]
    Debug --> Test
```

## Usage Timeline

```mermaid
gantt
    title Deminification Workflow Timeline
    dateFormat mm:ss

    section Setup
    Install tools           :00:00, 00:30
    Find Claude Code        :00:30, 00:10
    Deminify files          :00:40, 01:00
    Create indices          :01:40, 00:30

    section Analysis
    Search patterns         :02:10, 00:20
    Extract context         :02:30, 00:15
    Analyze code            :02:45, 01:00

    section Implementation
    Translate to Rust       :03:45, 01:30
    Write tests             :05:15, 01:00
    Verify parity           :06:15, 00:30

    section Documentation
    Document findings       :06:45, 00:15
```

## Pattern Search Decision Tree

```mermaid
flowchart TD
    Start([Need to understand<br/>Claude Code feature]) --> Know{Know exact feature?}

    Know -->|Yes| Direct[Direct search in deminified file]
    Know -->|No| Category{Know category?}

    Category -->|Yes| PrebuiltScript[Use search-patterns.sh]
    Category -->|No| Browse[Browse indices]

    Direct --> LineNo[Get line numbers]
    PrebuiltScript --> LineNo
    Browse --> LineNo

    LineNo --> Context[Extract context<br/>grep -B 10 -A 10]
    Context --> Understand{Understand logic?}

    Understand -->|No| More[Get more context<br/>expand range]
    Understand -->|Yes| Map[Map to Rust pattern]

    More --> Context

    Map --> Guide[Use translation guide]
    Guide --> Code[Write Rust code]
    Code --> Test[Test implementation]
    Test --> Match{Matches behavior?}

    Match -->|No| Debug[Debug differences]
    Match -->|Yes| Doc[Document in findings.md]

    Debug --> Context
    Doc --> End([Complete])
```

## File Dependency Graph

```mermaid
graph TD
    QS[QUICK_START_DEMINIFICATION.md] -->|References| DG[DEMINIFICATION_GUIDE.md]
    QS -->|Calls| AS[analyze-claude-code.sh]
    QS -->|Calls| SP[search-patterns.sh]

    DG -->|Documents| AS
    DG -->|Documents| SP
    DG -->|Shows examples from| CC[Claude Code cli.js]

    AS -->|Creates| PM[claude-code-minified.js]
    AS -->|Creates| JM[claude-code-jsbeautify.js]
    AS -->|Creates| Indices[index-*.txt]

    SP -->|Searches| PM
    SP -->|Uses| Indices

    RD[research/README.md] -->|Describes| PM
    RD -->|Describes| Indices
    RD -->|References| AS

    FIND[findings.md] -->|Templates for| Discoveries
    FIND -->|References| DG

    SUM[INVESTIGATION_SUMMARY.md] -->|Summarizes| All[All deliverables]
```

## Team Onboarding Flow

```mermaid
flowchart TD
    NewDev[New Developer] --> Read1[Read QUICK_START_DEMINIFICATION.md]
    Read1 --> Run[Run analyze-claude-code.sh]
    Run --> Try[Try example searches]
    Try --> Learn[Learn search-patterns.sh]
    Learn --> Browse[Browse research directory]
    Browse --> Read2[Read DEMINIFICATION_GUIDE.md]
    Read2 --> Practice[Practice JS→Rust translation]
    Practice --> Implement[Implement first feature]
    Implement --> Review[Review with team]
    Review --> Proficient[Proficient with workflow]
    Proficient --> Teach[Help next developer]
```

## Legend

### Symbols
- 📄 Document
- 🔧 Script/Tool
- 📁 Directory
- 🔍 Search Operation
- ⚙️ Process
- ✅ Deliverable
- 🎯 Goal

### Color Coding
- **Green**: Completed/Success
- **Blue**: Process/Action
- **Orange**: Decision Point
- **Red**: Error/Debug
- **Gray**: Reference/Documentation

## Usage Guide

### How to Use These Diagrams

1. **Overview Flowchart**: Shows complete workflow from start to finish
2. **Component Architecture**: Shows file relationships and dependencies
3. **Search Pattern Flow**: Guides through pattern search process
4. **Translation Pipeline**: Maps JavaScript patterns to Rust
5. **Usage Timeline**: Shows time estimates for each phase
6. **Decision Tree**: Helps choose right search approach
7. **Dependency Graph**: Shows document relationships
8. **Onboarding Flow**: Guides new team members

### Quick Reference

| Need | Use This Diagram |
|------|------------------|
| Starting out | Overview Flowchart |
| Understanding files | Component Architecture |
| Searching patterns | Search Pattern Flow |
| Translating code | Translation Pipeline |
| Time planning | Usage Timeline |
| Choosing method | Decision Tree |
| Finding docs | Dependency Graph |
| Onboarding | Team Onboarding Flow |

## Mermaid Rendering

These diagrams use Mermaid syntax. To view them:

1. **GitHub**: Renders automatically in markdown
2. **VS Code**: Install "Markdown Preview Mermaid Support" extension
3. **Online**: Use [mermaid.live](https://mermaid.live) editor
4. **CLI**: Use `mmdc` command from `@mermaid-js/mermaid-cli`

## Updating Diagrams

When workflow changes:

1. Update relevant diagram in this file
2. Test rendering in VS Code or GitHub
3. Update corresponding documentation
4. Commit with clear message about changes

## Additional Resources

- [Mermaid Documentation](https://mermaid.js.org/)
- [Flowchart Syntax](https://mermaid.js.org/syntax/flowchart.html)
- [Gantt Charts](https://mermaid.js.org/syntax/gantt.html)
