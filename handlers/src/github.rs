gh_pages_handlers!(
    [
        contacts_handler,
        "contact-info.json",
        types::contacts::Response
    ],
    [
        dictionary_handler,
        "dictionary.json",
        types::dictionary::Response
    ],
    [faqs_handler, "faqs.json", types::faqs::Response],
    [
        color_printers_handler,
        "color-printers.json",
        types::printing::Response
    ],
    [
        pause_menu_handler,
        "pause-menu.json",
        types::food::PauseMenuResponse
    ],
    [
        hours_handler,
        "building-hours.json",
        types::spaces::HoursResponse
    ],
    [help_handler, "help.json", types::tools::Response],
