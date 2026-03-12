
# jira

> **Fork Notice:** This project is a fork of [jira-terminal](https://github.com/amritghimire/jira-terminal) by Amrit Ghimire.
It has been renamed and adapted for personal use with Homebrew as the sole distribution method.

This application can be used for personal usage to manage Jira from the terminal.

## Installation

### Homebrew (macOS / Linux)

```bash
brew tap alienengineer/jira
brew install jira
```

## Autocompletion Script

To generate the autocompletion script, run:

```bash
jira autocompletion --shell [zsh|bash|fish|powershell|elvish] > _jira
```

Depending on your shell, you can move your autocompletion file to the following location:

- *ZSH* - `/usr/share/zsh/site-functions/_jira`
- *BASH* - `/usr/share/bash-completion/completions/_jira`
- *Fish* - `/share/fish/vendor_completions.d/_jira`

## Usage

When running the application for first time, you will be asked with following values.

- hostname [This will be used to identify the jira hostname to be used.]
- email [Email address you use to login with the application.]
- token [You can obtain the app password from the link specified in the application]

After that, you can use following commands for help.

```
jira help
jira help list
jira help transition
jira help alias
jira help detail
jira help fields
jira help update
jira help new
jira help assign
jira help comment
jira help autocompletion
jira help plugin
```

```
JIRA 2.4.2
alienengineer
This is a command line application that can be used as a personal productivity tool for interacting with JIRA

USAGE:
    jira [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    alias             Configuration for alias. One of add,list or remove is required.
    assign            Assign a ticket to user.
    autocompletion    Generate autocompletion script..
    comment           List or add comments to a ticket. Default action is adding.
    detail            Detail of a JIRA tickets..
    fields            List of possible Fields for details...
    help              Prints this message or the help of the given subcommand(s)
    list              List the issues from JIRA.
    new               Create a new ticket.
    plugin            Manage Lua plugins.
    transition        Transition of ticket across status.
    update            Update a field for a ticket
    logout            Erase configuration and log out of Jira
```

### List of Tickets

```

jira-list 
List the issues from JIRA.

USAGE:
    jira list [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -J, --json       JSON response
    -M, --me         Issues assigned to you.
    -V, --version    Prints version information

OPTIONS:
    -A, --alias <ALIAS>               Save the applied options as an alias. You can use it with jql option later.
    -a, --assignee <ASSIGNEE>...       Assignee username or email to filter with.
    -c, --component <COMPONENT>...    Component name or ID to filter with.
    -C, --count <COUNT>               Total number of issues to show. (Default is 50)
    -d, --display <DISPLAY>            Comma separated list of fields to display.
                                      Possible options for fields are:
                                      key,resolution,priority,assignee,status,components,creator,reporter,issuetype,project,summary
                                      
                                      You can pass alias as option for display. You can save alias using alias
                                      subcommand for the application.
                                      
                                       Default options are
                                       key,summary,status,assignee
                                                         
    -e, --epic <EPIC>...              EPIC name or issue key of epic to filter with.
    -f, --filter <FILTER>...          Filter name or filter id that you saved in JIRA.
    -j, --jql <JQL>                   JQL Query or alias to JQL query to filter with.
    -l, --label <LABEL>...            Search for issues with a label or list of labels.
    -o, --offset <OFFSET>             Offset to start the first item to return in a page of results. (Default is 0)
    -m, --main <PARENT>...            Search for subtask of a particular issue.
    -P, --priority <PRIORITY>...      Search for issues with a particular priority.
    -p, --project <PROJECT>...        Project Code to filter with.
    -r, --reporter <REPORTER>...      Search for issues that were reported by a particular user.
    -s, --sprint <SPRINT>...          Search for issues that are assigned to a particular sprint.
    -S, --status <STATUS>...          Search for issues that have a particular status.
    -T, --text <TEXT>                 This is a master-field that allows you to search all text fields for issues.
    -t, --type <TYPE>...              Search for issues that have a particular issue type. 

You can specify the following fields multiple time to filter by multiple values.
assignee, component, epic, filter, label, main, priority, project, reporter, sprint, status, type.

For example to fetch list of tickets in Backlog and In progress, you can use
jira list -s Backlog -s 'In Progress'
```

### Transition

```
jira-transition 
Transition of ticket across status.

USAGE:
    jira transition [FLAGS] <STATUS> --ticket <TICKET>

FLAGS:
    -h, --help       Prints help information
    -l, --list       List the possible transitions.
    -V, --version    Prints version information

OPTIONS:
    -t, --ticket <TICKET>    Ticket ID from JIRA.

ARGS:
    <STATUS>    Status or alias of status to move the ticket to.

```

### Alias

```
jira-alias 
Configuration for alias. One of add,list or remove is required.

USAGE:
    jira alias [FLAGS] <NAME> --add <add> --list --remove

FLAGS:
    -h, --help       Prints help information
    -l, --list       List the alias saved.
    -r, --remove     List the alias saved.
    -V, --version    Prints version information

OPTIONS:
    -a, --add <add>    Value to associate with provided alias name.

ARGS:
    <NAME>    Name of alias. (Required except for list option)
```

Sample usage:

- `jira alias -l`
- `jira alias alias_name -a "Alias Value"`
- `jira alias -r alias_name`

### Plugins

Install the bundled Lua plugins into your local plugins directory (`~/plugins`):

```bash
jira plugin generate
```

For compatibility, `jira plugin new` also works.

### Detail

```
jira-detail 
Detail of a JIRA tickets..

USAGE:
    jira detail [OPTIONS] <TICKET>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -f, --fields <fields>    Comma separated lists of fields or alias to show.
                             Possible options are: 
                             key,summary,description,status,issuetype,priority,labels,assignee,components,creator,reporter,project,comment
                             
                             You can use all to show all fields.
                             Default selection are:
                             key,summary,description
                                                 

ARGS:
    <TICKET>    Ticket id for details.

```

### Fields

```
jira-fields 
List of possible Fields for details...

USAGE:
    jira fields <TICKET>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <TICKET>    Ticket id for details.
```

### Update

```
jira-update 
Update a field for a ticket

USAGE:
    jira update <TICKET> --field <field> --value <value>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -f, --field <field>    Key of field to update. You can use jira fields <TICKET> to see possible set of keys.
    -v, --value <value>    Value of the field to update.

ARGS:
    <TICKET>    Ticket ID to update
```

### New

```
jira-new 
Create a new ticket.

USAGE:
    jira new [FLAGS] [OPTIONS] --main <main> --project <project>

FLAGS:
    -h, --help       Prints help information
    -M, --minimal    Only summary and description will be asked if not available.
    -q, --quiet      Do not ask for missing options.
    -V, --version    Prints version information

OPTIONS:
    -a, --assignee <assignee>          Assignee email of ticket
    -c, --components <components>      Comma separated list of components of ticket
    -C, --custom <custom>              Comma separated value pair for custom fields. You can use alias in value or key
                                       itself. Example- "customfield_12305:value,alias_to_key:value2. You can use fields
                                       subcommand to check the list of custom fields available. 
    -d, --description <description>    Description of ticket
    -l, --labels <labels>              Comma separated list of labels.
    -m, --main <main>                  Main ticket to create the sub-ticket.
    -p, --priority <priority>          Priority Of the ticket.
    -P, --project <project>            Project Key to create the ticket.
    -s, --summary <summary>            Summary of ticket
    -t, --type <type>                  Issue type for new ticket.
```

### Assign

```
jira-assign 
Assign a ticket to user.

USAGE:
    jira assign --ticket <ticket> --user <user>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -t, --ticket <ticket>    Ticket to use.
    -u, --user <user>        Assign the ticket to the provided user.
```

### Comment

```
jira-comment 
List or add comments to a ticket. Default action is adding.

USAGE:
    jira comment [FLAGS] [OPTIONS] --ticket <ticket>

FLAGS:
    -h, --help       Prints help information
    -l, --list       List all the comments of a ticket.
    -V, --version    Prints version information

OPTIONS:
    -b, --body <body>        Body of the comment. To mention someone, you can use @(query) The query can include jira
                             username or display name or email address.
    -t, --ticket <ticket>    Ticket to use.
```
