const originalError = console.error;

console.error = (...args: unknown[]) => {
  const msg = typeof args[0] === "string" ? args[0] : "";
  if (msg.includes("was not wrapped in act(...)")) {
    throw new Error(`React act(...) warning detected in test: ${msg}`);
  }
  originalError(...args);
};
