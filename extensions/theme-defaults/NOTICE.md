# Theme resource provenance

The declarative theme JSON files in this package are copied from the sibling VS Code source
tree (`extensions/theme-defaults`) and remain subject to the upstream Microsoft MIT license.
Only themes that are self-contained, without VS Code `include` dependencies, are packaged here.
The current extension service strictly validates their color/token rules, registers selectable
Workbench color themes, and projects the active theme's TextMate token rules. The upstream MIT
license text is shipped once at `zeta-resources/licenses/vscode/LICENSE.txt` by both package builders.
