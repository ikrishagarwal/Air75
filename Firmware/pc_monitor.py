import asyncio
import json
import serial
import serial.tools.list_ports
from winsdk.windows.media.control import GlobalSystemMediaTransportControlsSessionManager as MediaManager

# Try to import pycaw for Mic/Volume control
try:
    from comtypes import CLSCTX_ALL
    from pycaw.pycaw import AudioUtilities, IAudioEndpointVolume
    PYCAW_AVAILABLE = True
except ImportError:
    PYCAW_AVAILABLE = False
    print("Warning: 'pycaw' not installed. Mic status will be fake. Run: pip install pycaw comtypes")

# --- Configuration ---
TARGET_PORT = "COM8"  # Set definitively if known, or None to auto-detect
BAUD_RATE = 115200

async def get_media_info():
    try:
        sessions = await MediaManager.request_async()
        current_session = sessions.get_current_session()
        if current_session:
            info = await current_session.try_get_media_properties_async()
            title = info.title if info.title else "Unknown"
            artist = info.artist if info.artist else ""
            return f"{title} - {artist}"
        else:
            # Fallback: Check if any session is playing?
            # Windows usually has one 'current' session if media is active.
            return "No Active Media"
    except Exception as e:
        print(f"Media Error: {e}")
        return "Media Err"

def get_mic_status():
    # True Input Mute is hard. We'll read MASTER OUTPUT MUTE as a proxy
    # or just pretend "LIVE" if not available.
    # Alternatively, check if we can find a Capture device.
    if not PYCAW_AVAILABLE:
        return "LIVE (No Lib)"
    
    try:
        devices = AudioUtilities.GetSpeakers()
        interface = devices.Activate(
            IAudioEndpointVolume._iid_, CLSCTX_ALL, None)
        volume = interface.QueryInterface(IAudioEndpointVolume)
        is_muted = volume.GetMute()
        return "MUTED" if is_muted else "LIVE"
    except Exception:
         return "LIVE (Err)"

def find_port():
    ports = serial.tools.list_ports.comports()
    for port in ports:
        if "CircuitPython" in port.description or "Seriale" in port.description or (port.vid == 0x239A):
            return port.device
    return None

async def main():
    print("Searching for keyboard...")
    port = TARGET_PORT if TARGET_PORT else find_port()
    
    if not port:
        print("Keyboard not found! Set TARGET_PORT in script.")
        return

    print(f"Connecting to {port}...")
    try:
        ser = serial.Serial(port, BAUD_RATE, timeout=1)
        print("Connected! Sending data... (Press Ctrl+C to stop)")
        
        while True:
            # 1. Gather Info
            song = await get_media_info()
            mic = get_mic_status()
            
            # Print to console for verification
            print(f"\rStatus: [{mic}] Playing: {song[:30]:<30}", end="")
            
            # 2. Send JSON
            data = {"media": song[:20], "mic": mic}
            json_str = json.dumps(data) + "\n"
            ser.write(json_str.encode("utf-8"))
            
            await asyncio.sleep(0.5) 
            
    except serial.SerialException as e:
        print(f"\nSerial Error: {e}")
    except KeyboardInterrupt:
        print("\nStopping...")
    finally:
        if 'ser' in locals() and ser.is_open:
            ser.close()

if __name__ == "__main__":
    asyncio.run(main())
