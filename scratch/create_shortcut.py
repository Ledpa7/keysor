import os
import subprocess

def main():
    # Target and working directories
    target_path = r"C:\Users\wjdwl\.gemini\antigravity\scratch\14-Keysor\target\release\keysor.exe"
    working_dir = r"C:\Users\wjdwl\.gemini\antigravity\scratch\14-Keysor\target\release"
    shortcut_path = r"C:\Users\wjdwl\Desktop\Keysor.lnk"
    desktop_exe = r"C:\Users\wjdwl\Desktop\keysor.exe"

    # 1. Remove the binary from the desktop if it exists
    if os.path.exists(desktop_exe):
        try:
            os.remove(desktop_exe)
            print(f"Removed binary from desktop: {desktop_exe}")
        except Exception as e:
            print(f"Error removing {desktop_exe}: {e}")

    # 2. Create the shortcut via PowerShell (escaped inside python to avoid command line arg parsing issues)
    ps_command = f"""
    $sh = New-Object -ComObject WScript.Shell
    $link = $sh.CreateShortcut('{shortcut_path}')
    $link.TargetPath = '{target_path}'
    $link.WorkingDirectory = '{working_dir}'
    $link.Save()
    """
    
    try:
        subprocess.run(["powershell", "-Command", ps_command], check=True)
        print(f"Created shortcut at: {shortcut_path} -> {target_path}")
    except Exception as e:
        print(f"Error creating shortcut: {e}")

if __name__ == "__main__":
    main()
