# Theme resource provenance

The declarative theme JSON files in this package are copied from the sibling VS Code source
tree (`extensions/theme-defaults`) and remain subject to the upstream Microsoft MIT license.
Only themes that are self-contained, without VS Code `include` dependencies, are packaged here.
The current extension service validates and catalogs their color/token rules; activation
into Zeta's complete Workbench color-theme registry remains a separate integration boundary.
