# Build Instructions

## Dictionary Data Generation

The dictionary data (`dictionary-data/src/lib.rs`) is auto-generated and excluded from git to keep the repository size manageable. 

### Automatic Generation
All build commands automatically generate dictionary data if missing:
```bash
make tui      # Generates data if needed, then runs TUI
make web      # Generates data if needed, then runs web server  
make web-build # Generates data if needed, then builds web app
```

### Manual Generation
```bash
make codegen       # Generate full dictionary data
make codegen-test  # Generate limited test data (faster)
```

### Clean Up
```bash
make clean  # Cleans all build artifacts including dictionary data
```

## Development Workflow

1. **Fresh clone**: Just run `make tui` or `make web` - dictionary data generates automatically
2. **Updates**: Dictionary data persists across builds and git operations
3. **Clean builds**: Use `make clean` to remove all generated files

## Technical Details

- **Generated file**: `dictionary-data/src/lib.rs` (~113MB)
- **Source data**: JMDict JSON files (auto-downloaded during generation)  
- **Git handling**: File is tracked with `--skip-worktree` to ignore changes
- **Cleanup**: Auto-cleanup removes ~5GB of intermediate files after generation