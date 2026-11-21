# Holds the display (and the system) on for a timed desktop run, as the play
# harness's keeper does (crates/viewer/src/tools/play/display.rs): asks
# Windows for both (SetThreadExecutionState, ES_DISPLAY_REQUIRED |
# ES_SYSTEM_REQUIRED, continuous) and wakes the display by a pointer move of
# 0 pixels, which turns it on for as long as the request is held (the request
# alone, or SC_MONITORPOWER, does not on the PC the runs are made on). The
# pointer move is input: the idle time restarts, and the line printed names
# its time (UTC, as the keeper's) so a guard that reads the idle time after a
# run can tell it from a person's.
#
# With the display off after the power plan's idle timeout every present
# waits about 264 ms: the release's long play of 2026-11-20 (08:30, the PC
# unused since 07:28) had 6,807 frames of 263 ms, the GPU's work 17 ms each.
#
#   display-hold.ps1 -StopFile <path> [-Seconds N]
#
# Holds until StopFile exists or N seconds have passed (default 3600).
param([Parameter(Mandatory = $true)][string]$StopFile, [int]$Seconds = 3600)
$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class DisplayHold {
    [DllImport("kernel32.dll")]
    public static extern uint SetThreadExecutionState(uint flags);
    [StructLayout(LayoutKind.Sequential)]
    public struct MOUSEINPUT {
        public int dx; public int dy; public uint mouseData; public uint dwFlags;
        public uint time; public IntPtr dwExtraInfo;
    }
    [StructLayout(LayoutKind.Sequential)]
    public struct INPUT { public uint type; public MOUSEINPUT mi; }
    [DllImport("user32.dll", SetLastError = true)]
    public static extern uint SendInput(uint count, INPUT[] inputs, int size);
    // A relative pointer move of 0 pixels (INPUT_MOUSE, MOUSEEVENTF_MOVE).
    public static uint MoveZero() {
        INPUT[] input = new INPUT[1];
        input[0].type = 0;
        input[0].mi.dwFlags = 1;
        return SendInput(1, input, Marshal.SizeOf(typeof(INPUT)));
    }
}
"@
# ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED
$held = [DisplayHold]::SetThreadExecutionState([uint32]2147483651)
if ($held -eq 0) { "display-hold: the request failed"; exit 1 }
$sent = [DisplayHold]::MoveZero()
$at = (Get-Date).ToUniversalTime()
"display-hold: held on (ES_DISPLAY_REQUIRED); woken by a pointer move of 0 at {0:HH:mm:ss}.{1}Z (sent {2})" -f $at, [int][math]::Floor($at.Millisecond / 100), $sent
[Console]::Out.Flush()
$until = (Get-Date).AddSeconds($Seconds)
while (-not (Test-Path -LiteralPath $StopFile) -and (Get-Date) -lt $until) {
    Start-Sleep -Seconds 1
}
[void][DisplayHold]::SetThreadExecutionState([uint32]2147483648)
"display-hold: released at {0:HH:mm:ss}Z" -f (Get-Date).ToUniversalTime()
