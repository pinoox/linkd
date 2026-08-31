use crossterm::style::Stylize;

pub fn print_welcome_guide() {
    println!();
    println!(
        "  {} {}",
        "⚡ linkd".cyan().bold(),
        format!("v{}", env!("CARGO_PKG_VERSION")).dark_grey()
    );
    println!(
        "  {}",
        "Continuous local-dev package link daemon for multi-ecosystem monorepos".white()
    );
    println!();

    println!("  {}", "USAGE:".yellow().bold());
    println!(
        "    {} {} {}",
        "linkd".cyan().bold(),
        "<COMMAND>".green().bold(),
        "[OPTIONS]".dark_grey()
    );
    println!();

    println!("  {}", "🚀 GETTING STARTED & INTERACTIVE:".yellow().bold());
    print_cmd("init", "", "Guided interactive setup wizard (or `linkd wizard`)");
    print_cmd("doctor", "[--explain <topic>]", "Validate environment, permissions & store health");
    println!();

    println!("  {}", "📦 GLOBAL REUSABLE PACKAGES:".yellow().bold());
    print_cmd("register", "[path] (pin, add)", "Register current package globally for reuse");
    print_cmd("use", "<package> [consumer]", "Link a registered package into the current project");
    print_cmd("packages", "(pinned)", "List all globally registered packages");
    print_cmd("unregister", "<package> (unpin)", "Remove package from global registry");
    println!();

    println!("  {}", "🔗 DIRECT LINKING & STATE:".yellow().bold());
    print_cmd("link", "<source> [consumer]", "Link a local package directly into a project");
    print_cmd("unlink", "<package>", "Safely disconnect package link and clean markers");
    print_cmd("list", "", "List all active synchronized links across projects");
    print_cmd("status", "[--json]", "Check daemon health, process PID & sync state");
    println!();

    println!("  {}", "⚡ DAEMON & LIVE MONITORING:".yellow().bold());
    print_cmd("monitor", "(top, dashboard)", "Launch real-time interactive full-screen TUI");
    print_cmd("start", "", "Launch background continuous reconciler daemon");
    print_cmd("stop", "[--force]", "Gracefully stop the running background daemon");
    print_cmd("watch", "", "Run foreground link watcher with live event logs");
    print_cmd("logs", "[-f / --follow]", "Inspect or tail daemon event logs");
    println!();

    println!("  {}", "🛠️  UTILITIES & SHELL:".yellow().bold());
    print_cmd("version", "[-v, --json]", "Display version, target OS and ecosystem info");
    print_cmd("completions", "<bash|zsh|fish|pwsh>", "Generate shell auto-completion script");
    print_cmd("help", "[command]", "Print detailed help for a specific command");
    println!();

    println!("  {}", "💡 QUICK EXAMPLES:".yellow().bold());
    println!("    {}  {}", "$ linkd init".cyan(), "# Interactive setup wizard".dark_grey());
    println!("    {}  {}", "$ cd packages/ui && linkd register".cyan(), "# Register package globally".dark_grey());
    println!("    {}  {}", "$ cd apps/web && linkd use @acme/ui".cyan(), "# Use registered package in app".dark_grey());
    println!("    {}  {}", "$ linkd link ./packages/core ./apps/backend".cyan(), "# Direct local link".dark_grey());
    println!("    {}  {}", "$ linkd monitor".cyan(), "# Open live terminal dashboard".dark_grey());
    println!();

    println!("  {}", "🌐 ECOSYSTEMS & DOCUMENTATION:".yellow().bold());
    println!(
        "    {} {}",
        "Supported:".white().bold(),
        "JS/TS (npm/pnpm/yarn/bun) • PHP (Composer) • Python (uv/pip) • Go • Rust • JVM • Custom".dark_grey()
    );
    println!(
        "    {} {}",
        "Docs Hub: ".white().bold(),
        "https://pinoox.github.io/linkd/".cyan().underlined()
    );
    println!(
        "    {} {}",
        "GitHub:   ".white().bold(),
        "https://github.com/pinoox/linkd".cyan().underlined()
    );
    println!();
}

fn print_cmd(name: &str, args: &str, desc: &str) {
    let name_styled = name.green().bold();
    let name_pad = if name.len() < 13 { 13 - name.len() } else { 1 };

    let args_styled = if args.is_empty() {
        "".dark_grey()
    } else {
        args.dark_grey()
    };
    let args_pad = if args.len() < 24 { 24 - args.len() } else { 1 };

    println!(
        "    {}{:pad1$}{}{:pad2$}{}",
        name_styled,
        "",
        args_styled,
        "",
        desc.white(),
        pad1 = name_pad,
        pad2 = args_pad
    );
}
