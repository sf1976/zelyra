use super::*;

const INVOICE_SOURCE: &str = include_str!("../../examples/invoice_tutorial.zyl");

pub(super) fn command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let Some(name) = arguments.next() else {
        eprintln!("Usage: zelyra --tutorial invoice [directory]");
        return ExitCode::from(2);
    };
    if name != "invoice" {
        eprintln!("error[E-TUTORIAL-001]: unknown tutorial `{name}`; available tutorial: invoice");
        return ExitCode::from(2);
    }
    let directory = arguments
        .next()
        .unwrap_or_else(|| "invoice-tutorial".to_owned());
    if arguments.next().is_some() {
        eprintln!("Usage: zelyra --tutorial invoice [directory]");
        return ExitCode::from(2);
    }

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
    let result = create_project(&directory, options);
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
        println!("Invoice tutorial — your application is ready to explore.");
        println!("\nFollow these five steps in `{}`:", main_path.display());
        println!("  1. Customers: create and find customers at /customers.");
        println!("  2. Items: maintain reusable products and prices at /items.");
        println!("  3. Dashboard: review invoices at / and /views/invoicedashboard.");
        println!("  4. Invoice: create a draft at /invoices and select its customer.");
        println!("  5. Lines: add invoice items at /invoice_lines.");
        println!("\nStart the local database and application:");
        println!("  cd {}", directory);
        println!("  follow the MariaDB start and environment commands printed above");
        println!("  zelyra db setup main.zyl");
        println!("  zelyra serve main.zyl");
        println!("\nOpen http://127.0.0.1:3000. Use `zelyra editor .` to inspect the source.");
        println!("Learning scope: this starter records a user-entered total. Tax calculation, PDF output, sending, and production accounting rules are not included.");
    } else {
        println!("Rechnungstutorial — deine Anwendung kann erkundet werden.");
        println!(
            "\nArbeite diese fünf Schritte in `{}` durch:",
            main_path.display()
        );
        println!("  1. Kunden: unter /customers erfassen und suchen.");
        println!("  2. Artikel: Produkte und Preise unter /items pflegen.");
        println!("  3. Dashboard: Rechnungen unter / und /views/invoicedashboard ansehen.");
        println!("  4. Rechnung: unter /invoices einen Entwurf anlegen und Kunden auswählen.");
        println!("  5. Positionen: unter /invoice_lines Artikel zur Rechnung hinzufügen.");
        println!("\nLokale Datenbank und Anwendung starten:");
        println!("  cd {}", directory);
        println!("  die MariaDB-Start- und Umgebungsbefehle von oben ausführen");
        println!("  zelyra db setup main.zyl");
        println!("  zelyra serve main.zyl");
        println!(
            "\nÖffne http://127.0.0.1:3000. Mit `zelyra editor .` kannst du den Quelltext ansehen."
        );
        println!("Lernumfang: Der Gesamtbetrag wird eingetragen. Steuerberechnung, PDF, Versand und produktive Buchhaltungsregeln sind nicht enthalten.");
    }
    ExitCode::SUCCESS
}
