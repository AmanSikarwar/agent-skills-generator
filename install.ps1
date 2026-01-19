#Requires -Version 5.1
<#
.SYNOPSIS
    Agent Skills Generator Installer for Windows

.DESCRIPTION
    Downloads and installs the Agent Skills Generator CLI tool.

.PARAMETER InstallDir
    Installation directory. Defaults to $env:LOCALAPPDATA\Programs\agent-skills-generator

.PARAMETER Version
    Specific version to install. Defaults to latest.

.PARAMETER AddToPath
    Add installation directory to user PATH. Defaults to $true.

.EXAMPLE
    iwr -useb https://raw.githubusercontent.com/AmanSikarwar/agent-skills-generator/master/install.ps1 | iex

.EXAMPLE
    .\install.ps1 -Version v0.1.0 -InstallDir "C:\tools"

.LINK
    https://github.com/AmanSikarwar/agent-skills-generator
#>

[CmdletBinding()]
param(
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\agent-skills-generator",
    [string]$Version = "",
    [bool]$AddToPath = $true
)

$ErrorActionPreference = "Stop"

# Configuration
$Repo = "AmanSikarwar/agent-skills-generator"
$BinaryName = "agent-skills-generator"

# Colors
function Write-ColorOutput {
    param(
        [string]$Type,
        [string]$Message
    )

    switch ($Type) {
        "Info"    { Write-Host "INFO " -ForegroundColor Blue -NoNewline; Write-Host $Message }
        "Success" { Write-Host "SUCCESS " -ForegroundColor Green -NoNewline; Write-Host $Message }
        "Warn"    { Write-Host "WARN " -ForegroundColor Yellow -NoNewline; Write-Host $Message }
        "Error"   { Write-Host "ERROR " -ForegroundColor Red -NoNewline; Write-Host $Message }
    }
}

function Write-Banner {
    $banner = @"

    _                    _     ____  _    _ _ _
   / \   __ _  ___ _ __ | |_  / ___|| | _(_) | |___
  / _ \ / _` |/ _ \ '_ \| __| \___ \| |/ / | | / __|
 / ___ \ (_| |  __/ | | | |_   ___) |   <| | | \__ \
/_/   \_\__, |\___|_| |_|\__| |____/|_|\_\_|_|_|___/
        |___/
              Generator Installer (Windows)

"@
    Write-Host $banner -ForegroundColor Cyan
}

function Get-Architecture {
    $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    switch ($arch) {
        "X64"   { return "x86_64" }
        "Arm64" { return "aarch64" }
        default { throw "Unsupported architecture: $arch" }
    }
}

function Get-LatestVersion {
    $releases = "https://api.github.com/repos/$Repo/releases/latest"
    try {
        $response = Invoke-RestMethod -Uri $releases -Method Get -ErrorAction Stop
        return $response.tag_name
    }
    catch {
        throw "Failed to fetch latest version: $_"
    }
}

function Get-DownloadUrl {
    param(
        [string]$Version,
        [string]$Arch
    )

    $target = "$Arch-pc-windows-msvc"
    return "https://github.com/$Repo/releases/download/$Version/$BinaryName-$Version-$target.zip"
}

function Add-ToUserPath {
    param(
        [string]$Directory
    )

    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")

    if ($currentPath -split ";" -contains $Directory) {
        Write-ColorOutput "Info" "Directory already in PATH"
        return
    }

    $newPath = "$currentPath;$Directory"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    $env:Path = "$env:Path;$Directory"

    Write-ColorOutput "Success" "Added to user PATH: $Directory"
}

function Install-AgentSkillsGenerator {
    Write-Banner

    # Detect architecture
    Write-ColorOutput "Info" "Detecting system..."
    $arch = Get-Architecture
    Write-ColorOutput "Info" "Architecture: $arch"

    # Get version
    if ([string]::IsNullOrEmpty($Version)) {
        Write-ColorOutput "Info" "Fetching latest version..."
        $Version = Get-LatestVersion
    }
    Write-ColorOutput "Info" "Version: $Version"

    # Create install directory
    Write-ColorOutput "Info" "Installation directory: $InstallDir"
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }

    # Get download URL
    $downloadUrl = Get-DownloadUrl -Version $Version -Arch $arch
    Write-ColorOutput "Info" "Downloading from: $downloadUrl"

    # Create temp directory
    $tempDir = Join-Path $env:TEMP "agent-skills-generator-install-$(Get-Random)"
    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

    try {
        # Download
        $zipPath = Join-Path $tempDir "$BinaryName.zip"
        Write-ColorOutput "Info" "Downloading..."

        $ProgressPreference = 'SilentlyContinue'
        Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath -UseBasicParsing

        # Extract
        Write-ColorOutput "Info" "Extracting..."
        Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force

        # Install
        Write-ColorOutput "Info" "Installing..."
        $binaryPath = Join-Path $tempDir "$BinaryName.exe"
        $destPath = Join-Path $InstallDir "$BinaryName.exe"

        Copy-Item -Path $binaryPath -Destination $destPath -Force

        # Add to PATH
        if ($AddToPath) {
            Add-ToUserPath -Directory $InstallDir
        }

        # Verify installation
        Write-ColorOutput "Success" "Successfully installed $BinaryName $Version"
        Write-Host ""
        Write-ColorOutput "Info" "Run '$BinaryName --help' to get started"

        # Check if new terminal needed
        $currentPath = $env:Path -split ";"
        if ($currentPath -notcontains $InstallDir) {
            Write-Host ""
            Write-ColorOutput "Warn" "Please restart your terminal for PATH changes to take effect"
        }
    }
    finally {
        # Cleanup
        if (Test-Path $tempDir) {
            Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

# Run installer
Install-AgentSkillsGenerator
