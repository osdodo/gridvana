use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoCreateInstance,
    CoInitializeEx,
};
use windows::Win32::UI::Shell::{
    DestinationList, ICustomDestinationList, SetCurrentProcessExplicitAppUserModelID,
};
use windows::core::HSTRING;

const APP_ID: &str = "com.gridvana.app";

pub fn initialize() {
    unsafe {
        let app_id = HSTRING::from(APP_ID);
        let _ = SetCurrentProcessExplicitAppUserModelID(&app_id);
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);

        // Versions before the tray menu registered user tasks in the Windows
        // Jump List. Remove that persisted list so the obsolete launchers do
        // not remain after upgrading.
        if let Ok(destination_list) = CoCreateInstance::<_, ICustomDestinationList>(
            &DestinationList,
            None,
            CLSCTX_INPROC_SERVER,
        ) {
            let _ = destination_list.DeleteList(&app_id);
        }
    }
}
