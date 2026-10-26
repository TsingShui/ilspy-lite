#!/usr/bin/env pwsh
# Build ILSpy.slnx, or an explicitly selected solution. Defaults to Debug.
# Extra arguments are forwarded to `dotnet build` (e.g. ./build.ps1 -Configuration Release --no-restore).
param(
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Debug',
    [string]$Solution = 'ILSpy.slnx'
)
$ErrorActionPreference = 'Stop'
$env:OPENSSL_ENABLE_SHA1_SIGNATURES = '1'
dotnet build -mt $Solution -c $Configuration "-p:Platform=Any CPU" @args
exit $LASTEXITCODE
