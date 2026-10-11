use super::*;

const INVOICE_SOURCE: &str = include_str!("../../examples/invoice_tutorial.zyl");
const INVOICE_THEME: &str = include_str!("../../examples/invoice_tutorial.theme.css");

pub(super) fn command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let Some(name) = arguments.next() else {
        eprintln!("Usage: zelyra --tutorial invoice [directory] [--no-start]");
        return ExitCode::from(2);
    };
    if name != "invoice" {
        eprintln!("error[E-TUTORIAL-001]: unknown tutorial `{name}`; available tutorial: invoice");
        return ExitCode::from(2);
    }
    let mut directory = None;
    let mut no_start = false;
    for argument in arguments {
        if argument == "--no-start" && !no_start {
            no_start = true;
        } else if !argument.starts_with('-') && directory.is_none() {
            directory = Some(argument);
        } else {
            eprintln!("Usage: zelyra --tutorial invoice [directory] [--no-start]");
            return ExitCode::from(2);
        }
    }
    let directory = directory.unwrap_or_else(|| "invoice-tutorial".to_owned());

    let options = ProjectOptions {
        allow_current_directory: false,
        with_mariadb: true,
        crud_template: false,
        auth_template: false,
        business_template: false,
        web_port: DEFAULT_WEB_PORT,
        host_port: DEFAULT_WEB_PORT,
        database_host_port: DEFAULT_DATABASE_HOST_PORT,
        host_port_given: false,
        database_host_port_given: false,
    };
    let result = create_project_with_setup_hint(&directory, options, false);
    if result != ExitCode::SUCCESS {
        return result;
    }

    let main_path = PathBuf::from(&directory).join("main.zyl");
    if let Err(error) = fs::write(&main_path, INVOICE_SOURCE) {
        eprintln!(
            "error[E-TUTORIAL-002]: could not write tutorial application `{}`: {error}",
            main_path.display()
        );
        return ExitCode::FAILURE;
    }
    let theme_path = PathBuf::from(&directory).join("zelyra.theme.css");
    if let Err(error) = fs::write(&theme_path, INVOICE_THEME) {
        eprintln!(
            "error[E-TUTORIAL-004]: could not write tutorial theme `{}`: {error}",
            theme_path.display()
        );
        return ExitCode::FAILURE;
    }
    let source_path = main_path.to_string_lossy();
    if validate(&source_path).is_err() {
        eprintln!(
            "error[E-TUTORIAL-003]: generated invoice tutorial did not pass its compiler check"
        );
        return ExitCode::FAILURE;
    }

    let english =
        env::var("ZELYRA_LANGUAGE").is_ok_and(|language| language.eq_ignore_ascii_case("en"));
    println!();
    if english {
        println!(
            "{}",
            if no_start {
                "Invoice tutorial — the project scaffold is ready."
            } else {
                "Invoice tutorial — setting up your local application now."
            }
        );
        println!("\nFollow these five steps in `{}`:", main_path.display());
        println!("  1. Customers: create and find customers at /customers.");
        println!("  2. Items: maintain reusable products and prices at /items.");
        println!("  3. Dashboard: review invoices at / and /views/invoicedashboard.");
        println!("  4. Invoice: create a draft at /invoices and select its customer.");
        println!("  5. Lines: add invoice items at /invoice_lines.");
        if no_start {
            println!("\nAutomatic startup was skipped. To start the database, apply the schema, and launch the app, run:");
            println!("  cd {directory} && zelyra setup --all");
        } else {
            println!(
                "\nStarting the local database and preparing the application automatically..."
            );
            match setup_action(&directory, "all", &SetupOptions::default()) {
                Ok(details) => println!("{details}"),
                Err(error) => {
                    eprintln!(
                        "error[E-TUTORIAL-005]: could not prepare the invoice tutorial: {error}"
                    );
                    eprintln!("The project files are ready. After Docker Compose is available, run `cd {directory} && zelyra setup --all`.");
                    return ExitCode::FAILURE;
                }
            }
        }
        println!("Open Studio with `zelyra editor {directory}` to inspect the source.");
        println!("The invoice page calculates net, 19% VAT, and gross totals from its lines. Open /invoices/INVOICE-NUMBER/print and use the browser print dialog to save a PDF.");
        println!("The example assumes the German standard rate; it is not a complete legally compliant invoicing or accounting system.");
    } else {
        println!(
            "{}",
            if no_start {
                "Rechnungstutorial — dein Projektgerüst ist bereit."
            } else {
                "Rechnungstutorial — deine lokale Anwendung wird jetzt eingerichtet."
            }
        );
        println!(
            "\nArbeite diese fünf Schritte in `{}` durch:",
            main_path.display()
        );
        println!("  1. Kunden: unter /customers erfassen und suchen.");
        println!("  2. Artikel: Produkte und Preise unter /items pflegen.");
        println!("  3. Dashboard: Rechnungen unter / und /views/invoicedashboard ansehen.");
        println!("  4. Rechnung: unter /invoices einen Entwurf anlegen und Kunden auswählen.");
        println!("  5. Positionen: unter /invoice_lines Artikel zur Rechnung hinzufügen.");
        if no_start {
            println!("\nAutomatischer Start übersprungen. Mit diesem Befehl startest du Datenbank und Anwendung und richtest das Schema ein:");
            println!("  cd {directory} && zelyra setup --all");
        } else {
            println!("\nLokale Datenbank starten und Anwendung automatisch vorbereiten...");
            match setup_action(&directory, "all", &SetupOptions::default()) {
                Ok(details) => println!("{details}"),
                Err(error) => {
                    eprintln!("error[E-TUTORIAL-005]: das Rechnungstutorial konnte nicht gestartet werden: {error}");
                    eprintln!("Die Projektdateien sind bereit. Sobald Docker Compose verfügbar ist: `cd {directory} && zelyra setup --all`.");
                    return ExitCode::FAILURE;
                }
            }
        }
        println!("Mit `zelyra editor {directory}` kannst du den Quelltext ansehen.");
        println!("Die Rechnungsseite berechnet Netto, 19% Umsatzsteuer und Brutto aus den Positionen. Öffne /invoices/RECHNUNGSNUMMER/print und speichere über den Druckdialog des Browsers als PDF.");
        println!("Das Beispiel nimmt den allgemeinen deutschen Steuersatz an und ist kein vollständiges gesetzeskonformes Rechnungs- oder Buchhaltungssystem.");
    }
    ExitCode::SUCCESS
}
