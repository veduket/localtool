# Profiles

Profiles let you maintain multiple isolated DNS configurations and switch between them.

## Use case

Imagine you work on two projects:

- **Project A**: `api.project-a.test`, `frontend.project-a.test`
- **Project B**: `api.project-b.dev`, `admin.project-b.dev`

With profiles, you can keep them separate and switch instantly:

```bash
# Project A
localtool dns profile switch project-a
localtool dns add api.project-a.test 127.0.0.1

# Switch to Project B (Project A's entries are hidden)
localtool dns profile switch project-b
localtool dns add api.project-b.dev 127.0.0.1

# Go back to Project A
localtool dns profile switch project-a
```

## Commands

### Create a profile

```bash
localtool dns profile create work
```

### Switch active profile

```bash
localtool dns profile switch work
```

### List profiles

```bash
localtool dns profile list
```

### Delete a profile

```bash
localtool dns profile delete work
```

This deletes the profile and all its associated zones, groups, and entries.

## Default profile

When you run `localtool dns init`, a default profile is created. All entries added without specifying a profile go into the active profile.
