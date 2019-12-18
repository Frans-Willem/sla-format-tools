use gio::prelude::*;
use gtk::prelude::*;

fn build_ui(application: &gtk::Application) {
    // let glade_src = include_str!("sl1topws_ui.glade");
    let builder = gtk::Builder::new_from_file("./src/bin/sl1topws_ui.glade");//new_from_string(glade_src);

    let window : gtk::ApplicationWindow = builder.get_object("window").unwrap();
    window.set_application(Some(application));

    window.show_all();

}
fn build_ui2(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);

    window.set_title("Prusa SL1 to Anycubic PWS converter");
    window.set_border_width(10);
    window.set_position(gtk::WindowPosition::Center);
    window.set_default_size(350, 70);

    let input_file = gtk::FileChooserButton::new("Input file", gtk::FileChooserAction::Open);
    let input_file_filter = gtk::FileFilter::new();
    input_file_filter.set_name(Some("Prusa SL1 file (*.sl1)"));
    input_file_filter.add_pattern("*.sl1");
    input_file.add_filter(&input_file_filter);

    let go_button = gtk::Button::new_with_label("Convert");
    let window_clone = window.clone();
    go_button.connect_clicked(move |btn| {
        let output_file = gtk::FileChooserDialog::with_buttons(
            Some("Save to"),
            Some(&gtk::Window::new(gtk::WindowType::Popup)),
            gtk::FileChooserAction::Save,
            &[("_Cancel", gtk::ResponseType::Cancel), ("_Save", gtk::ResponseType::Accept)]
        );
        let output_file_filter = gtk::FileFilter::new();
        output_file_filter.set_name(Some("Anycubic Photon PWS file (*.pws)"));
        output_file_filter.add_pattern("*.pws");
        output_file.add_filter(&output_file_filter);
        let response = output_file.run();
        output_file.emit_close();
        println!("Response: {:?}", response);
    });

    let container = gtk::Grid::new();
    container.set_row_spacing(8);
    container.set_column_spacing(8);
    container.attach(&gtk::Label::new(Some("Input SL1:")), 0, 0, 1, 1);
    input_file.set_hexpand(true);
    container.attach(&input_file,1,0,1,1);

    container.attach(&gtk::Label::new(Some("Anti-aliasing:")), 0, 1, 1, 1);

    go_button.set_hexpand(true);
    container.attach(&go_button,0,2,2,1);
    window.add(&container);

    window.show_all();
}

fn main() {
    let application = gtk::Application::new(Some("nl.hardijzer.fw.sl1topws"), Default::default())
        .expect("Application::new failed");

    application.connect_activate(|app| {
        build_ui(app);
    });

    application.run(&std::env::args().collect::<Vec<_>>());
}
