# CLI API Reference

## Overview

The CLI API provides the command-line interface for Ralph Orchestrator, including commands, arguments, and shell integration.

## Main CLI Interface

### RalphCLI Class

```python
class RalphCLI:
    """
    Main CLI interface for Ralph Orchestrator.
    
    Example:
        cli = RalphCLI()
        cli.run(sys.argv[1:])
    """
    
    def __init__(self):
        """Initialize CLI with command registry."""
        self.commands = {
            'run': self.cmd_run,
            'init': self.cmd_init,
            'state': self.cmd_state,
            'clean': self.cmd_clean,
            'config': self.cmd_config,
            'agents': self.cmd_agents,
            'metrics': self.cmd_metrics,
            'checkpoint': self.cmd_checkpoint,
            'rollback': self.cmd_rollback,
            'help': self.cmd_help
        }
        self.parser = self.create_parser()
    
    def create_parser(self) -> argparse.ArgumentParser:
        """
        Create argument parser.
        
        Returns:
            ArgumentParser: Configured parser
        """
        parser = argparse.ArgumentParser(
            prog='ralph',
            description='Ralph Orchestrator - AI task automation',
            formatter_class=argparse.RawDescriptionHelpFormatter,
            epilog="""
Examples:
  ralph run                    # Run with auto-detected agent
  ralph run -a claude          # Run with Claude
  ralph run -a acp             # Run with ACP agent
  ralph run -a acp --acp-agent gemini --acp-permission-mode auto_approve
  ralph state status           # Show runtime workflow state summaries
  ralph clean                  # Clean workspace
  ralph init                   # Initialize new project
            """
        )
        
        # Global arguments
        parser.add_argument(
            '--version',
            action='version',
            version='%(prog)s 1.0.0'
        )
        
        parser.add_argument(
            '--verbose', '-v',
            action='store_true',
            help='Enable verbose output'
        )
        
        parser.add_argument(
            '--config', '-c',
            help='Configuration file path'
        )
        
        # Subcommands
        subparsers = parser.add_subparsers(
            dest='command',
            help='Available commands'
        )
        
        # Run command
        run_parser = subparsers.add_parser(
            'run',
            help='Run orchestrator'
        )
        run_parser.add_argument(
            '--agent', '-a',
            choices=['claude', 'q', 'gemini', 'acp', 'auto'],
            default='auto',
            help='AI agent to use'
        )
        run_parser.add_argument(
            '--acp-agent',
            default='gemini',
            help='ACP agent command (for -a acp)'
        )
        run_parser.add_argument(
            '--acp-permission-mode',
            choices=['auto_approve', 'deny_all', 'allowlist', 'interactive'],
            default='auto_approve',
            help='Permission handling mode for ACP agent'
        )
        run_parser.add_argument(
            '--prompt', '-p',
            default='PROMPT.md',
            help='Prompt file path'
        )
        run_parser.add_argument(
            '--max-iterations', '-i',
            type=int,
            default=100,
            help='Maximum iterations'
        )
        run_parser.add_argument(
            '--dry-run',
            action='store_true',
            help='Test mode without execution'
        )
        
        # Init command
        subparsers.add_parser(
            'init',
            help='Initialize new project'
        )
        
        # State command
        state_parser = subparsers.add_parser(
            'state',
            help='Inspect or clear runtime workflow state'
        )
        state_subparsers = state_parser.add_subparsers(
            dest='state_command',
            help='Runtime state commands'
        )
        state_subparsers.add_parser(
            'status',
            help='Show runtime workflow state summaries'
        )
        read_parser = state_subparsers.add_parser(
            'read',
            help='Read one runtime workflow state record'
        )
        read_parser.add_argument(
            'mode',
            help='State mode: ralph, ralplan, team, deep-interview, capability-invocation'
        )
        clear_parser = state_subparsers.add_parser(
            'clear',
            help='Clear runtime workflow state for one mode'
        )
        clear_parser.add_argument(
            'mode',
            help='State mode: ralph, ralplan, team, deep-interview, capability-invocation'
        )
        
        # Clean command
        subparsers.add_parser(
            'clean',
            help='Clean workspace'
        )
        
        # Config command
        config_parser = subparsers.add_parser(
            'config',
            help='Manage configuration'
        )
        config_parser.add_argument(
            'action',
            choices=['show', 'set', 'get'],
            help='Configuration action'
        )
        config_parser.add_argument(
            'key',
            nargs='?',
            help='Configuration key'
        )
        config_parser.add_argument(
            'value',
            nargs='?',
            help='Configuration value'
        )
        
        # Agents command
        subparsers.add_parser(
            'agents',
            help='List available agents'
        )
        
        # Metrics command
        metrics_parser = subparsers.add_parser(
            'metrics',
            help='View metrics'
        )
        metrics_parser.add_argument(
            '--format',
            choices=['text', 'json', 'csv'],
            default='text',
            help='Output format'
        )
        
        # Checkpoint command
        checkpoint_parser = subparsers.add_parser(
            'checkpoint',
            help='Create checkpoint'
        )
        checkpoint_parser.add_argument(
            '--message', '-m',
            help='Checkpoint message'
        )
        
        # Rollback command
        rollback_parser = subparsers.add_parser(
            'rollback',
            help='Rollback to checkpoint'
        )
        rollback_parser.add_argument(
            'checkpoint',
            nargs='?',
            help='Checkpoint ID or "last"'
        )
        
        return parser
    
    def run(self, args: List[str] = None):
        """
        Run CLI with arguments.
        
        Args:
            args (list): Command line arguments
            
        Returns:
            int: Exit code
            
        Example:
            cli = RalphCLI()
            exit_code = cli.run(['run', '--agent', 'claude'])
        """
        args = self.parser.parse_args(args)
        
        # Setup logging
        if args.verbose:
            logging.basicConfig(level=logging.DEBUG)
        else:
            logging.basicConfig(level=logging.INFO)
        
        # Load configuration
        if args.config:
            config = load_config(args.config)
        else:
            config = load_config()
        
        # Execute command
        if args.command:
            command = self.commands.get(args.command)
            if command:
                return command(args, config)
            else:
                print(f"Unknown command: {args.command}")
                return 1
        else:
            self.parser.print_help()
            return 0
```

## Command Implementations

### Run Command

```python
def cmd_run(self, args, config):
    """
    Execute the run command.
    
    Args:
        args: Parsed arguments
        config: Configuration dictionary
        
    Returns:
        int: Exit code
        
    Example:
        cli.cmd_run(args, config)
    """
    # Update config with CLI arguments
    if args.agent:
        config['agent'] = args.agent
    if args.prompt:
        config['prompt_file'] = args.prompt
    if args.max_iterations:
        config['max_iterations'] = args.max_iterations
    if args.dry_run:
        config['dry_run'] = True
    
    # Create and run orchestrator
    orchestrator = RalphOrchestrator(config)
    
    try:
        result = orchestrator.run()
        
        if result['success']:
            print(f"✓ Task completed in {result['iterations']} iterations")
            return 0
        else:
            print(f"✗ Task failed: {result.get('error', 'Unknown error')}")
            return 1
            
    except KeyboardInterrupt:
        print("\n⚠ Interrupted by user")
        return 130
    except Exception as e:
        print(f"✗ Error: {str(e)}")
        return 1
```

### Init Command

```python
def cmd_init(self, args, config):
    """
    Initialize new Ralph project.
    
    Args:
        args: Parsed arguments
        config: Configuration dictionary
        
    Returns:
        int: Exit code
        
    Example:
        cli.cmd_init(args, config)
    """
    print("Initializing Ralph Orchestrator project...")
    
    # Create directories
    directories = ['.agent', '.ralph/state', '.ralph/diagnostics']
    for directory in directories:
        os.makedirs(directory, exist_ok=True)
        print(f"  ✓ Created {directory}")
    
    # Create default PROMPT.md
    if not os.path.exists('PROMPT.md'):
        with open('PROMPT.md', 'w') as f:
            f.write("""# Task Description

Describe your task here...

## Requirements
- [ ] Requirement 1
- [ ] Requirement 2

## Success Criteria
- The task is complete when...

<!-- Ralph will continue iterating until limits are reached -->
""")
        print("  ✓ Created PROMPT.md template")
    
    # Create default config
    if not os.path.exists('ralph.json'):
        with open('ralph.json', 'w') as f:
            json.dump({
                'agent': 'auto',
                'max_iterations': 100,
                'checkpoint_interval': 5
            }, f, indent=2)
        print("  ✓ Created ralph.json config")
    
    # Initialize Git if not present
    if not os.path.exists('.git'):
        subprocess.run(['git', 'init'], capture_output=True)
        print("  ✓ Initialized Git repository")
    
    print("\n✓ Project initialized successfully!")
    print("\nNext steps:")
    print("  1. Edit PROMPT.md with your task")
    print("  2. Run: ralph run")
    
    return 0
```

### State Command

```python
def cmd_state(self, args, config):
    """
    Inspect runtime workflow state.
    
    Args:
        args: Parsed arguments
        config: Configuration dictionary
        
    Returns:
        int: Exit code
        
    Example:
        cli.cmd_state(args, config)
    """
    if args.state_command == 'status':
        print("mode: ralph")
        print("  exists: true")
        print("  active: false")
        print("  current_phase: -")
        print("  run_outcome: -")
        print("  lifecycle_outcome: -")
        print("  path: ./.ralph/state/ralph-state.json")
        return 0

    if args.state_command == 'read':
        state_file = f'.ralph/state/{args.mode}-state.json'
        if not os.path.exists(state_file):
            print(f"No runtime workflow state found for mode {args.mode}")
            return 1
        with open(state_file) as f:
            print(f.read())
        return 0

    if args.state_command == 'clear':
        state_file = f'.ralph/state/{args.mode}-state.json'
        if os.path.exists(state_file):
            os.remove(state_file)
        print(f"Cleared runtime workflow state for mode {args.mode}")
        return 0

    # Check Git working tree from diagnostics scripts, not as a runtime state source.
    result = subprocess.run(
        ['git', '-C', '.'] + 'status --porcelain'.split(),
        capture_output=True,
        text=True
    )
    if result.stdout:
        print(f"\n⚠ Uncommitted changes present")
    else:
        print(f"\n✓ Git: clean working directory")
    
    return 0
```

### Clean Command

```python
def cmd_clean(self, args, config):
    """
    Clean Ralph workspace.
    
    Args:
        args: Parsed arguments
        config: Configuration dictionary
        
    Returns:
        int: Exit code
        
    Example:
        cli.cmd_clean(args, config)
    """
    print("Cleaning Ralph workspace...")
    
    # Confirm before cleaning
    response = input("This will remove all Ralph data. Continue? [y/N]: ")
    if response.lower() != 'y':
        print("Cancelled")
        return 0
    
    # Clean directories
    directories = [
        '.ralph/state',
        '.ralph/diagnostics'
    ]
    
    for directory in directories:
        if os.path.exists(directory):
            shutil.rmtree(directory)
            os.makedirs(directory)
            print(f"  ✓ Cleaned {directory}")
    
    # Reset state
    state = StateManager()
    state.reset()
    print("  ✓ Reset state")
    
    print("\n✓ Workspace cleaned successfully!")
    
    return 0
```

## Shell Integration

### Bash Completion

```bash
# ralph-completion.bash
_ralph_completion() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    
    # Main commands
    opts="run init state clean config agents metrics checkpoint rollback help"
    
    case "${prev}" in
        ralph)
            COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
            return 0
            ;;
        --agent|-a)
            COMPREPLY=( $(compgen -W "claude q gemini acp auto" -- ${cur}) )
            return 0
            ;;
        --acp-agent)
            COMPREPLY=( $(compgen -c -- ${cur}) )
            return 0
            ;;
        --acp-permission-mode)
            COMPREPLY=( $(compgen -W "auto_approve deny_all allowlist interactive" -- ${cur}) )
            return 0
            ;;
        --format)
            COMPREPLY=( $(compgen -W "text json csv" -- ${cur}) )
            return 0
            ;;
        config)
            COMPREPLY=( $(compgen -W "show set get" -- ${cur}) )
            return 0
            ;;
    esac
    
    # File completion for prompt files
    if [[ ${cur} == *.md ]]; then
        COMPREPLY=( $(compgen -f -X '!*.md' -- ${cur}) )
        return 0
    fi
    
    COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
}

complete -F _ralph_completion ralph
```

### ZSH Completion

```zsh
# ralph-completion.zsh
#compdef ralph

_ralph() {
    local -a commands
    commands=(
        'run:Run orchestrator'
        'init:Initialize project'
        'state:Inspect runtime workflow state'
        'clean:Clean workspace'
        'config:Manage configuration'
        'agents:List agents'
        'metrics:View metrics'
        'checkpoint:Create checkpoint'
        'rollback:Rollback to checkpoint'
        'help:Show help'
    )
    
    _arguments \
        '--version[Show version]' \
        '--verbose[Enable verbose output]' \
        '--config[Configuration file]:file:_files' \
        '1:command:->command' \
        '*::arg:->args'
    
    case $state in
        command)
            _describe 'command' commands
            ;;
        args)
            case $words[1] in
                run)
                    _arguments \
                        '--agent[AI agent]:agent:(claude q gemini acp auto)' \
                        '--prompt[Prompt file]:file:_files -g "*.md"' \
                        '--max-iterations[Max iterations]:number' \
                        '--acp-agent[ACP agent command]:command' \
                        '--acp-permission-mode[Permission mode]:mode:(auto_approve deny_all allowlist interactive)' \
                        '--dry-run[Test mode]'
                    ;;
                config)
                    _arguments \
                        '1:action:(show set get)' \
                        '2:key' \
                        '3:value'
                    ;;
            esac
            ;;
    esac
}
```

## Interactive Mode

```python
class InteractiveCLI:
    """
    Interactive CLI mode for Ralph.
    
    Example:
        interactive = InteractiveCLI()
        interactive.run()
    """
    
    def __init__(self):
        self.running = True
        self.orchestrator = None
        self.config = load_config()
    
    def run(self):
        """Run interactive mode."""
        print("Ralph Orchestrator Interactive Mode")
        print("Type 'help' for commands, 'exit' to quit")
        print()
        
        while self.running:
            try:
                command = input("ralph> ").strip()
                if command:
                    self.execute_command(command)
            except KeyboardInterrupt:
                print("\nUse 'exit' to quit")
            except EOFError:
                self.running = False
    
    def execute_command(self, command: str):
        """Execute interactive command."""
        parts = command.split()
        cmd = parts[0]
        args = parts[1:] if len(parts) > 1 else []
        
        commands = {
            'help': self.cmd_help,
            'run': self.cmd_run,
            'state': self.cmd_state,
            'stop': self.cmd_stop,
            'config': self.cmd_config,
            'agents': self.cmd_agents,
            'exit': self.cmd_exit,
            'quit': self.cmd_exit
        }
        
        if cmd in commands:
            commands[cmd](args)
        else:
            print(f"Unknown command: {cmd}")
    
    def cmd_help(self, args):
        """Show help."""
        print("""
Available commands:
  run [agent]    - Start orchestrator
  state status   - Show runtime workflow state summaries
  stop           - Stop orchestrator
  config [key]   - Show/set configuration
  agents         - List available agents
  help           - Show this help
  exit           - Exit interactive mode
        """)
    
    def cmd_exit(self, args):
        """Exit interactive mode."""
        if self.orchestrator:
            print("Stopping orchestrator...")
            # Stop orchestrator
        print("Goodbye!")
        self.running = False
```

## Plugin System

```python
class CLIPlugin:
    """
    Base class for CLI plugins.
    
    Example:
        class MyPlugin(CLIPlugin):
            def register_commands(self, cli):
                cli.add_command('mycommand', self.my_command)
    """
    
    def __init__(self, name: str):
        self.name = name
    
    def register_commands(self, cli: RalphCLI):
        """Register plugin commands with CLI."""
        raise NotImplementedError
    
    def register_arguments(self, parser: argparse.ArgumentParser):
        """Register plugin arguments."""
        pass

class PluginManager:
    """Manage CLI plugins."""
    
    def __init__(self):
        self.plugins = []
    
    def load_plugin(self, plugin: CLIPlugin):
        """Load a plugin."""
        self.plugins.append(plugin)
    
    def register_all(self, cli: RalphCLI):
        """Register all plugins with CLI."""
        for plugin in self.plugins:
            plugin.register_commands(cli)
```
