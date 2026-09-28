param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("store", "read", "metadata", "delete")]
    [string]$Operation,

    [Parameter(Mandatory = $true)]
    [ValidatePattern("^AgentBrowser/[A-Za-z0-9/_-]{1,160}$")]
    [string]$Target
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public static class AgentBrowserCredentialManager
{
    public const UInt32 CRED_TYPE_GENERIC = 1;
    public const UInt32 CRED_PERSIST_LOCAL_MACHINE = 2;
    public const int ERROR_NOT_FOUND = 1168;

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct NativeCredential
    {
        public UInt32 Flags;
        public UInt32 Type;
        [MarshalAs(UnmanagedType.LPWStr)] public string TargetName;
        [MarshalAs(UnmanagedType.LPWStr)] public string Comment;
        public System.Runtime.InteropServices.ComTypes.FILETIME LastWritten;
        public UInt32 CredentialBlobSize;
        public IntPtr CredentialBlob;
        public UInt32 Persist;
        public UInt32 AttributeCount;
        public IntPtr Attributes;
        [MarshalAs(UnmanagedType.LPWStr)] public string TargetAlias;
        [MarshalAs(UnmanagedType.LPWStr)] public string UserName;
    }

    [DllImport("advapi32.dll", EntryPoint = "CredWriteW", CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool CredWrite(ref NativeCredential credential, UInt32 flags);

    [DllImport("advapi32.dll", EntryPoint = "CredReadW", CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool CredRead(string target, UInt32 type, UInt32 flags, out IntPtr credential);

    [DllImport("advapi32.dll", EntryPoint = "CredDeleteW", CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool CredDelete(string target, UInt32 type, UInt32 flags);

    [DllImport("advapi32.dll", EntryPoint = "CredFree", SetLastError = false)]
    public static extern void CredFree(IntPtr buffer);

    public static UInt64 FileTimeValue(System.Runtime.InteropServices.ComTypes.FILETIME value)
    {
        return ((UInt64)(UInt32)value.dwHighDateTime << 32) | (UInt32)value.dwLowDateTime;
    }
}
"@

try {
    if ($Operation -eq "store") {
        $inputStream = [Console]::OpenStandardInput()
        $bytes = [byte[]]::new(2560)
        $handle = $null
        try {
            $length = 0
            while ($length -lt $bytes.Length) {
                $count = $inputStream.Read($bytes, $length, $bytes.Length - $length)
                if ($count -eq 0) {
                    break
                }
                $length += $count
            }
            if ($length -lt 1 -or ($length -eq $bytes.Length -and $inputStream.ReadByte() -ne -1)) {
                throw "credential_payload_invalid"
            }
            $handle = [Runtime.InteropServices.GCHandle]::Alloc(
                $bytes,
                [Runtime.InteropServices.GCHandleType]::Pinned
            )
            $credential = [AgentBrowserCredentialManager+NativeCredential]::new()
            $credential.Type = [AgentBrowserCredentialManager]::CRED_TYPE_GENERIC
            $credential.TargetName = $Target
            $credential.Comment = "Agent Browser private SAM Login.gov credential"
            $credential.CredentialBlobSize = $length
            $credential.CredentialBlob = $handle.AddrOfPinnedObject()
            $credential.Persist = [AgentBrowserCredentialManager]::CRED_PERSIST_LOCAL_MACHINE
            $credential.UserName = "agent-browser-private"
            if (-not [AgentBrowserCredentialManager]::CredWrite([ref]$credential, 0)) {
                throw "credential_write_failed"
            }
        }
        finally {
            if ($null -ne $handle -and $handle.IsAllocated) {
                $handle.Free()
            }
            [Array]::Clear($bytes, 0, $bytes.Length)
        }
        [Console]::Out.Write("stored")
        exit 0
    }

    if ($Operation -eq "delete") {
        if (-not [AgentBrowserCredentialManager]::CredDelete(
            $Target,
            [AgentBrowserCredentialManager]::CRED_TYPE_GENERIC,
            0
        )) {
            $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
            if ($code -ne [AgentBrowserCredentialManager]::ERROR_NOT_FOUND) {
                throw "credential_delete_failed"
            }
        }
        [Console]::Out.Write("deleted")
        exit 0
    }

    $pointer = [IntPtr]::Zero
    $found = [AgentBrowserCredentialManager]::CredRead(
        $Target,
        [AgentBrowserCredentialManager]::CRED_TYPE_GENERIC,
        0,
        [ref]$pointer
    )
    if (-not $found) {
        $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        if ($code -eq [AgentBrowserCredentialManager]::ERROR_NOT_FOUND -and $Operation -eq "metadata") {
            [Console]::Out.Write('{"present":false}')
            exit 0
        }
        throw "credential_read_failed"
    }
    try {
        $credential = [Runtime.InteropServices.Marshal]::PtrToStructure(
            $pointer,
            [type][AgentBrowserCredentialManager+NativeCredential]
        )
        if ($Operation -eq "metadata") {
            $ticks = [AgentBrowserCredentialManager]::FileTimeValue($credential.LastWritten)
            [Console]::Out.Write((@{
                present = $true
                last_written_filetime = $ticks.ToString([Globalization.CultureInfo]::InvariantCulture)
            } | ConvertTo-Json -Compress))
            exit 0
        }
        if ($credential.CredentialBlobSize -lt 1 -or $credential.CredentialBlobSize -gt 2560) {
            throw "credential_payload_invalid"
        }
        $bytes = [byte[]]::new($credential.CredentialBlobSize)
        try {
            [Runtime.InteropServices.Marshal]::Copy(
                $credential.CredentialBlob,
                $bytes,
                0,
                $credential.CredentialBlobSize
            )
            $output = [Console]::OpenStandardOutput()
            $output.Write($bytes, 0, $bytes.Length)
            $output.Flush()
        }
        finally {
            [Array]::Clear($bytes, 0, $bytes.Length)
        }
    }
    finally {
        [AgentBrowserCredentialManager]::CredFree($pointer)
    }
}
catch {
    [Console]::Error.WriteLine("private_windows_credential_store_failed")
    exit 1
}
