# The PC's load beside a timed run (scripts/long-play.sh): one line a second
# with the whole machine's CPU, the disks, the 3D engine of the graphics card
# and the busiest processes that are not the run's own, so that a slow frame
# can be read against what else the PC did in that second (the release's
# long play of 2026-11-23: one frame of 157 ms, render prepare, and nothing
# in the run's own record to say whether the PC or the game made it).
#
#   pc-load.ps1 -Out <path> -StopFile <path> [-Seconds N] [-Ours name,name]
#
# Each line:
#   L <unix ms> cpu <total %> disk <read MB/s> <write MB/s> gpu <3D %> self <%> ours <%> faults <ours' page faults/s> top <name> <pid> <% of one core> ...
#   G <unix ms> gpu-top <name> <pid> <3D %> ...   (a process not ours using the 3D engine, 3 % or more)
# The time is the sample's end; a sample covers about the second before it.
# "self" is this sampler's own CPU, "ours" the -Ours processes' (% of one
# core); names have their spaces as "_".
# CPU per process is the share of one core (100 = one core busy all second),
# as Windows' process counters give it; the processes named in -Ours (without
# .exe, the run's viewer) and this script itself are left out of the "top"
# lists but counted in the totals. Reads counters only: changes nothing.
param(
    [Parameter(Mandatory = $true)][string]$Out,
    [Parameter(Mandatory = $true)][string]$StopFile,
    [int]$Seconds = 3600,
    [string[]]$Ours = @()
)
$ErrorActionPreference = 'Continue'
$me = $PID
# powershell -File hands "a,b" over as one string.
$Ours = @($Ours | ForEach-Object { $_ -split ',' } | Where-Object { $_ })
$counters = @(
    '\Processor(_Total)\% Processor Time',
    '\PhysicalDisk(_Total)\Disk Read Bytes/sec',
    '\PhysicalDisk(_Total)\Disk Write Bytes/sec',
    '\Process(*)\% Processor Time',
    '\Process(*)\ID Process',
    '\GPU Engine(*engtype_3D)\Utilization Percentage'
)
# The -Ours processes' page faults (all their instances): a frame that
# allocates megabytes fresh touches new pages, and a busy memory manager
# slows each fault.
foreach ($o in $Ours) { $counters += "\Process($o*)\Page Faults/sec" }
$writer = [System.IO.StreamWriter]::new($Out, $false)
$writer.AutoFlush = $true
$writer.WriteLine("# pc-load: started $([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()), ours: $($Ours -join ','), sampler pid $me")
$end = (Get-Date).AddSeconds($Seconds)
$ourSet = @{}
foreach ($o in $Ours) { $ourSet[$o.ToLowerInvariant()] = $true }
# A counter query of 5 samples at a time: a query expands "Process(*)"
# to the processes alive when it starts (with -Continuous a process started
# later, the run's viewer among them, is never seen), and a new query a
# second costs about a third of a second of its own.
while ($true) {
Get-Counter -Counter $counters -SampleInterval 1 -MaxSamples 5 -ErrorAction SilentlyContinue | ForEach-Object {
    if ((Get-Date) -ge $end -or (Test-Path -LiteralPath $StopFile)) {
        $writer.WriteLine("# pc-load: stopped $([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds())")
        $writer.Close()
        exit 0
    }
    $s = $_
    $ms = [DateTimeOffset]::new($s.Timestamp).ToUnixTimeMilliseconds()
    $cpu = 0.0; $read = 0.0; $write = 0.0
    $pct = @{}; $ids = @{}; $gpu = @{}
    foreach ($c in $s.CounterSamples) {
        $p = $c.Path
        if ($p -like '*\processor(_total)\*') { $cpu = $c.CookedValue }
        elseif ($p -like '*disk read bytes/sec') { $read = $c.CookedValue }
        elseif ($p -like '*disk write bytes/sec') { $write = $c.CookedValue }
        elseif ($p -like '*\process(*)\% processor time') { $pct[$c.InstanceName] = $c.CookedValue }
        elseif ($p -like '*\process(*)\id process') { $ids[$c.InstanceName] = [int]$c.CookedValue }
        elseif ($p -like '*\process(*)\page faults/sec') { $faults += $c.CookedValue }
        elseif ($p -like '*\gpu engine(*') {
            # Instance names read pid_<n>_luid_..._engtype_3d.
            if ($c.InstanceName -match '^pid_(\d+)_') {
                $g = [int]$Matches[1]
                $gpu[$g] = [double]($gpu[$g]) + $c.CookedValue
            }
        }
    }
    $names = @{}
    $self = 0.0
    $mine = 0.0
    $faults = 0.0
    $rows = foreach ($k in $pct.Keys) {
        if ($k -eq '_total' -or $k -eq 'idle') { continue }
        $id = $ids[$k]
        $name = ($k -replace '#\d+$', '') -replace '\s', '_'
        if ($id) { $names[$id] = [string]$name }
        if ($id -eq $me) { $self = $pct[$k] }
        if ($ourSet.ContainsKey("$name".ToLowerInvariant())) { $mine += $pct[$k] }
        if ($id -eq $me -or $ourSet.ContainsKey("$name".ToLowerInvariant())) { continue }
        [pscustomobject]@{ Name = $name; Id = $id; Pct = $pct[$k] }
    }
    $top = $rows | Where-Object { $_.Pct -ge 5 } | Sort-Object Pct -Descending | Select-Object -First 5
    $gpuAll = 0.0
    foreach ($g in $gpu.Keys) { $gpuAll += $gpu[$g] }
    $line = 'L {0} cpu {1:F0} disk {2:F1} {3:F1} gpu {4:F0} self {5:F0} ours {6:F0} faults {7:F0} top' -f $ms, $cpu, ($read / 1MB), ($write / 1MB), $gpuAll, $self, $mine, $faults
    foreach ($t in $top) { $line += (' {0} {1} {2:F0}' -f $t.Name, $t.Id, $t.Pct) }
    $writer.WriteLine($line)
    $foreign = foreach ($g in $gpu.Keys) {
        $n = [string]$names[$g]
        if (-not $n) { $n = '?' }
        if ($gpu[$g] -lt 3 -or $ourSet.ContainsKey("$n".ToLowerInvariant())) { continue }
        [pscustomobject]@{ Name = $n; Id = $g; Pct = $gpu[$g] }
    }
    if ($foreign) {
        $gl = "G $ms gpu-top"
        foreach ($t in ($foreign | Sort-Object Pct -Descending | Select-Object -First 3)) {
            $gl += (' {0} {1} {2:F0}' -f $t.Name, $t.Id, $t.Pct)
        }
        $writer.WriteLine($gl)
    }
}
    if ((Get-Date) -ge $end -or (Test-Path -LiteralPath $StopFile)) { break }
}
$writer.WriteLine("# pc-load: stopped $([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds())")
$writer.Close()
