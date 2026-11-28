# Windows' licence service's starts in a run's time, for scripts/long-play.sh:
# ClipSVC, trigger-started by the Store's licence poll about every 15
# minutes (docs/test-coverage.md, "The long play"). The first line says the
# log was read ("read N"), then a line per start: the unix ms and the message.
# A log that could not be read writes nothing and exits 1, so the check can
# tell "no start in the run" from "not known".
param(
  [Parameter(Mandatory = $true)][long]$Start,
  [Parameter(Mandatory = $true)][long]$End
)
$filter = @{
  LogName   = 'Microsoft-Client-Licensing-Platform/Admin'
  Id        = 100
  StartTime = [DateTimeOffset]::FromUnixTimeSeconds($Start).LocalDateTime
  EndTime   = [DateTimeOffset]::FromUnixTimeSeconds($End).LocalDateTime
}
try {
  $events = @(Get-WinEvent -FilterHashtable $filter -ErrorAction Stop)
} catch {
  # Get-WinEvent reports "no event in the time" as an error of its own.
  if ($_.FullyQualifiedErrorId -like 'NoMatchingEventsFound*') {
    $events = @()
  } else {
    [Console]::Error.WriteLine("licence-events: $($_.Exception.Message)")
    exit 1
  }
}
"read $($events.Count)"
$events | Sort-Object TimeCreated | ForEach-Object {
  '{0} {1}' -f ([DateTimeOffset]$_.TimeCreated).ToUnixTimeMilliseconds(), ($_.Message -replace '\s+', ' ')
}
