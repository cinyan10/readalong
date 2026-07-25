import React from "react";
import ReactDOM from "react-dom/client";
import { Toaster } from "sonner";

import App from "./App";
import "./styles.css";
import { ThemeProvider, useTheme } from "@/lib/theme";

function ThemeToaster() {
  const { resolvedTheme } = useTheme();
  return <Toaster richColors closeButton position="bottom-right" theme={resolvedTheme} />;
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ThemeProvider>
      <App />
      <ThemeToaster />
    </ThemeProvider>
  </React.StrictMode>,
);
