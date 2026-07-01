import os
import shutil
import urllib.request

def main():
    url = "https://github.com/petoncle/mousemaster/releases/download/88/mousemaster.exe"
    target_dir = r"C:\Users\wjdwl\Downloads\mousemaster-main\mousemaster-main"
    dest_exe = os.path.join(target_dir, "mousemaster.exe")
    dest_props = os.path.join(target_dir, "mousemaster.properties")
    src_props = os.path.join(target_dir, "configuration", "neo-mousekeys-wasd.properties")

    # 1. Download mousemaster.exe
    print(f"Downloading mousemaster.exe from {url}...")
    try:
        # User-Agent header to avoid potential rate limit blocking by GitHub
        req = urllib.request.Request(
            url, 
            headers={'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)'}
        )
        with urllib.request.urlopen(req) as response, open(dest_exe, 'wb') as out_file:
            shutil.copyfileobj(response, out_file)
        print("Download completed successfully!")
    except Exception as e:
        print(f"Error downloading executable: {e}")
        return

    # 2. Copy and rename configuration file (default to WASD configuration)
    if os.path.exists(src_props):
        try:
            shutil.copyfile(src_props, dest_props)
            print(f"Configuration copied to: {dest_props}")
        except Exception as e:
            print(f"Error copying configuration: {e}")
    else:
        print(f"Warning: Configuration template not found at {src_props}")

if __name__ == "__main__":
    main()
