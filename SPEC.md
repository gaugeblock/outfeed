# Behavioral Specifications & Invariants

## Formal Type Contracts

- **`release::ApplyArgs`** (`struct`): Arguments for `outfeed apply`.
- **`sync_model::BranchDivergence`** (`enum`): Branch alignment status relative to upstream tracking remote.
- **`model::BumpType`** (`enum`): SemVer bump classification.
- **`hygiene::CleanArgs`** (`struct`): Arguments for `outfeed clean`.
- **`model::CleanOpts`** (`struct`): Options controlling workspace cache cleaning.
- **`model::CleanReport`** (`struct`): Summary report returned after a clean or prune operation.
- **`mod::Cli`** (`struct`): Outfeed — Neurosymbolic release gate, AST drift inspector & changelog synthesizer.
- **`clone::CloneArgs`** (`struct`): Arguments for `outfeed clone`.
- **`clone::CloneOpts`** (`struct`): Execution options for suite clone commands.
- **`clone::CloneStatus`** (`enum`): Status of a repository clone operation.
- **`clone::CloneSummary`** (`struct`): Summary result for a single repository clone operation.
- **`cmd::CommandOutput`** (`struct`): Captured output from a child process execution.
- **`mod::Commands`** (`enum`): Available subcommands in outfeed.
- **`model::CommitInfo`** (`struct`): Parsed git commit metadata.
- **`model::ConventionalKind`** (`enum`): Conventional commit classification.
- **`model::DiskUsage`** (`struct`): Breakdown of disk usage across target directories.
- **`clone::GitProtocol`** (`enum`): Git protocol used for cloning suite repositories.
- **`delivery::InstallArgs`** (`struct`): Arguments for `outfeed install`.
- **`model::InstallOpts`** (`struct`): Options controlling CLI binary installation.
- **`install::InstallOutcome`** (`enum`): Result of installing a tool.
