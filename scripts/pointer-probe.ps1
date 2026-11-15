# Run by scripts/input-guard.sh: does this desktop's pointer follow
# SetCursorPos? Prints "free X Y" or "pinned X Y" (the pointer's position).
# Only when the pointer sits at the primary screen's centre (where Synergy
# holds it while the other computer has the input) is it moved, by 150 px,
# watched for half a second and put back.
Add-Type -AssemblyName System.Windows.Forms
Add-Type -Namespace S2 -Name Pointer -MemberDefinition @'
[System.Runtime.InteropServices.DllImport("user32.dll")]
public static extern bool SetCursorPos(int x, int y);
'@
$screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$cx = $screen.X + [int][math]::Floor($screen.Width / 2)
$cy = $screen.Y + [int][math]::Floor($screen.Height / 2)
$p = [System.Windows.Forms.Cursor]::Position
if ([math]::Abs($p.X - $cx) -gt 2 -or [math]::Abs($p.Y - $cy) -gt 2) {
  "free $($p.X) $($p.Y)"
  exit 0
}
$tx = $cx - 150
$ty = $cy - 150
[void][S2.Pointer]::SetCursorPos($tx, $ty)
$held = $false
foreach ($wait in 100, 150, 250) {
  Start-Sleep -Milliseconds $wait
  $q = [System.Windows.Forms.Cursor]::Position
  if ($q.X -ne $tx -or $q.Y -ne $ty) { $held = $true; break }
}
[void][S2.Pointer]::SetCursorPos($p.X, $p.Y)
if ($held) { "pinned $($q.X) $($q.Y)" } else { "free $($p.X) $($p.Y)" }
