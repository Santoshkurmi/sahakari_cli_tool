# System Update Cli tool

```
Usage: sahakari_cli <COMMAND>

Commands:
  update  Update all or selected Laravel projects
  logs    Show logs of previous operations
  config  Configure the tool
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```
```
Update all or selected Laravel projects

Usage: sahakari_cli update [OPTIONS] [PATH]

Arguments:
  [PATH]  Update current directory,use dot(.) to update all projects

Options:
  -a, --all          Process all projects without prompting
      --only <ONLY>  Process only specific project(s) Not working
  -e, --errors       Show all projects with errors to update
      --dry-run      Show what would be done without executing
  -v, --verbose      Show detailed output
  -f, --force        Force update to run all commands even if no change
  -h, --help         Print help
```
