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
