# Zones & Groups

Zones and Groups let you organize DNS entries hierarchically within a profile.

## Hierarchy

```
Profile
  └── Zone (e.g., "addis_ababa")
        └── Group (e.g., "bole")
              └── DNS Entries
```

## Use case

If you're building services across multiple locations or teams:

```bash
# Create zones
localtool dns zone create addis_ababa -d "Addis Ababa"
localtool dns zone create dire_dawa -d "Dire Dawa"

# Create groups within zones
localtool dns group create bole --zone addis_ababa
localtool dns group create kazanchis --zone addis_ababa
localtool dns group create sabian --zone dire_dawa

# Add entries to specific groups
localtool dns add web.bole.test 127.0.0.1 -z addis_ababa -g bole
localtool dns add api.kazanchis.test 127.0.0.1 -z addis_ababa -g kazanchis
```

## Default zone/group

When you don't specify a zone or group, entries go into `global/main` — created automatically by `init`.

## Commands

### Zone commands

```bash
# Create a zone
localtool dns zone create addis_ababa -d "Addis Ababa"

# List all zones
localtool dns zone list

# Show zone details
localtool dns zone show addis_ababa

# Delete a zone (cascades to groups and entries)
localtool dns zone delete addis_ababa
```

### Group commands

```bash
# Create a group within a zone
localtool dns group create bole --zone addis_ababa

# List groups
localtool dns group list
localtool dns group list --zone addis_ababa

# Delete a group (cascades to entries)
localtool dns group delete bole --zone addis_ababa
```

## Cascade behavior

Deleting a zone deletes all its groups and their entries. Deleting a group deletes all its entries. This prevents orphaned data.
