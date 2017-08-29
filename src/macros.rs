// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#[macro_export]
macro_rules! cdp_default_port {
    () => ( 9222 )
}

#[macro_export]
macro_rules! cdp_frontend_url_format {
    () => (
        "chrome-devtools://devtools/bundled/inspector.html?ws={server_addr}/devtools/page/{page_id}"
    )
}

#[macro_export]
macro_rules! cdp_ws_path {
    () => ( "devtools/page")
}

#[macro_export]
macro_rules! cdp_ws_path_format {
    () => ( concat!(cdp_ws_path!(), "/{page_id}") )
}

#[macro_export]
macro_rules! cdp_ws_url_format {
    () => ( concat!("ws://{server_addr}/", cdp_ws_path_format!()) )
}

#[macro_export]
macro_rules! cdp_greeter_root_path {
    () => ( "json" )
}

#[macro_export]
macro_rules! cdp_greeter_version_info_slug {
    () => ( "version" )
}

#[macro_export]
macro_rules! cdp_greeter_version_info_path {
    () => ( concat!(cdp_greeter_root_path!(), "/", cdp_greeter_version_info_slug!()) )
}

#[macro_export]
macro_rules! cdp_greeter_version_info_url_format {
    () => ( concat!("http://{server_addr}/", cdp_greeter_version_info_path!()) )
}

#[macro_export]
macro_rules! cdp_greeter_page_list_slug {
    () => ( "list" )
}

#[macro_export]
macro_rules! cdp_greeter_page_list_path {
    () => ( concat!(cdp_greeter_root_path!(), "/", cdp_greeter_page_list_slug!()) )
}

#[macro_export]
macro_rules! cdp_greeter_page_list_url_format {
    () => ( concat!("http://{server_addr}/", cdp_greeter_page_list_path!()) )
}

#[macro_export]
macro_rules! cdp_greeter_new_page_slug {
    () => ( "new" )
}

#[macro_export]
macro_rules! cdp_greeter_new_page_path {
    () => ( concat!(cdp_greeter_root_path!(), "/", cdp_greeter_new_page_slug!()) )
}

#[macro_export]
macro_rules! cdp_greeter_new_page_and_navigate_path_format {
    () => ( concat!(cdp_greeter_new_page_path!(), "?{url}") )
}

#[macro_export]
macro_rules! cdp_greeter_new_page_url_format {
    () => ( concat!("http://{server_addr}/", cdp_greeter_new_page_path!()) )
}

#[macro_export]
macro_rules! cdp_greeter_new_page_and_navigate_url_format {
    () => ( concat!("http://{server_addr}/", cdp_greeter_new_page_and_navigate_path_format!()) )
}

#[macro_export]
macro_rules! cdp_greeter_activate_page_slug {
    () => ( "activate" )
}

#[macro_export]
macro_rules! cdp_greeter_activate_page_path {
    () => ( concat!(cdp_greeter_root_path!(), "/", cdp_greeter_activate_page_slug!()) )
}

#[macro_export]
macro_rules! cdp_greeter_activate_page_path_format {
    () => ( concat!(cdp_greeter_activate_page_path!(), "/{page_id}") )
}

#[macro_export]
macro_rules! cdp_greeter_activate_page_url_format {
    () => ( concat!("http://{server_addr}/", cdp_greeter_activate_page_path_format!()) )
}
