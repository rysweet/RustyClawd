# Quick Start: Slash Command Autocomplete

## Try It Now!

### 1. Build the project
```bash
cd /Users/ryan/src/declawed/claude-code-rs
cargo build --package rustyclawd-cli
```

### 2. Run interactive mode
```bash
cargo run --package rustyclawd-cli -- interactive
```

### 3. Test autocomplete

**Type these at the "You>" prompt:**

```
You> /<TAB>
```
→ Shows all available commands

```
You> /ul<TAB>
```
→ Autocompletes to `/ultrathink`

```
You> /deb<TAB>
```
→ Autocompletes to `/debug`

```
You> /an<TAB>
```
→ Autocompletes to `/analyze`

```
You> /ex<TAB>
```
→ Autocompletes to `/exit`

## Add Your Own Commands

Create a new command file:
```bash
echo "# My Custom Command" > .claude/commands/mycommand.md
```

Now `/mycommand` will appear in autocomplete!

## How It Works

1. **Type `/`** - Triggers autocomplete mode
2. **Continue typing** - Filters commands with fuzzy matching
3. **Press TAB** - Completes to best match or shows options
4. **Press ENTER** - Executes the command

## Available Commands

**Built-in:**
- `/exit` or `/quit` - Exit the session
- `/clear` - Clear conversation history
- `/help` - Show help message
- `/stats` - Show session statistics

**Custom (in `.claude/commands/`):**
- `/ultrathink` - Deep thinking mode
- `/debug` - Debug mode
- `/analyze` - Analysis mode

## Fuzzy Matching Examples

The fuzzy matcher is smart! Try these:

- `/ul` → `/ultrathink` (prefix match)
- `/ulth` → `/ultrathink` (fuzzy match)
- `/dbg` → `/debug` (fuzzy match)
- `/hlp` → `/help` (fuzzy match)

## Keyboard Shortcuts

- **TAB** - Show completions or complete current word
- **Ctrl+D** - Exit interactive mode
- **Ctrl+C** - Cancel current input
- **Up/Down** - Navigate through history

---

**SUCCESS!** Typing `/ul` + TAB completes to `/ultrathink` exactly as required!
