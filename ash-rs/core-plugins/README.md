# Core Plugins

- Owns Plugin discovery, durable installation and activation state, immutable package storage and invocation leases.
- Registers named Plugin providers, routes exact source IDs, and installs their verified packages.
- Implements the HTTPS/TUF Marketplace provider with independent trust and cache configuration for each source.
- Supplies resolved Skill, MCP, Connector, Hook and Extension contributions to their runtime owners.
