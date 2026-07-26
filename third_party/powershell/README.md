# PowerShell runtime

Zeta first uses the Windows-provided `powershell.exe`. A Windows release that
requires PowerShell 7 semantics may stage a pinned `pwsh.exe` runtime here using
the same download, checksum, license, and signing policy as PDFium.

Do not add `pwsh.exe` to source control. The terminal/process capability must
resolve an approved absolute executable path and remain behind Zeta's approval
and sandbox boundary.
