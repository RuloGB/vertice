# Delta for Inventory UI

## ADDED Requirements

### Requirement: The Log File Path Is Displayed As Selectable Text On The Scan Route

The `scan` route MUST render the absolute path of the application log file, obtained from the
log-path command, as selectable text alongside a localized label. The element MUST NOT provide a
"reveal in file manager" action or any button that opens the file or its containing folder — it MUST
only display the path for the user to copy.

#### Scenario: The log path is visible and selectable on the scan route

- GIVEN a successful invocation of the log-path command
- WHEN the user navigates to the `scan` route
- THEN the absolute log-file path is rendered as selectable text with a localized label
- AND no reveal-in-file-manager or file-opening action is present

#### Scenario: The rendered path matches what the command returns

- GIVEN the log-path command returns a specific absolute path
- WHEN the `scan` route renders the log-path element
- THEN the displayed text is exactly that path, unmodified
