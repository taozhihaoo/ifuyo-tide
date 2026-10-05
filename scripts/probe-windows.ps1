Start-Process -FilePath 'src-tauri\target\release\ifuyo-tide.exe'
Start-Sleep -Seconds 10
$p = Get-Process ifuyo-tide -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $p) { Write-Output 'DEAD'; exit 1 }
Write-Output ("pid=" + $p.Id + " handle=" + $p.MainWindowHandle + " title=[" + $p.MainWindowTitle + "]")
Add-Type @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class E {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder sb, int max);
  public static List<string> Find(uint target) {
    var found = new List<string>();
    EnumWindows(delegate(IntPtr h, IntPtr l) {
      uint pid; GetWindowThreadProcessId(h, out pid);
      if (pid == target) {
        var sb = new StringBuilder(256); GetWindowText(h, sb, 256);
        found.Add(h + "|vis=" + IsWindowVisible(h) + "|[" + sb + "]");
      }
      return true;
    }, IntPtr.Zero);
    return found;
  }
}
'@
[E]::Find([uint32]$p.Id) | ForEach-Object { Write-Output $_ }
Stop-Process -Id $p.Id -Force
