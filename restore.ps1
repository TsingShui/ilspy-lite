#!/usr/bin/env pwsh
# Restore ILSpy.slnx, or an explicitly selected solution, honouring packages.lock.json files.
# Use updatedeps.ps1 instead when you have changed a PackageVersion and need to refresh the locks.
param([string]$Solution = 'ILSpy.slnx')
$ErrorActionPreference = 'Stop'
$env:OPENSSL_ENABLE_SHA1_SIGNATURES = '1'
dotnet restore $Solution -p:RestoreEnablePackagePruning=false @args
exit $LASTEXITCODE
