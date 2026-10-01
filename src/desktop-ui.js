/**
 * BENDR Desktop UI Extensions
 * 
 * Desktop-specific UI panels and controls.
 * Only activates when running inside Tauri.
 */

if (window.__TAURI_INTERNALS__) {
  console.log('[BENDR Desktop] Desktop UI extensions loaded');
  
  // ── 1. Display Selector UI ────────────────────────────────
  const btnPop = document.getElementById("btnPop");
  
  if (btnPop) {
    // Modify button tooltip for desktop context
    btnPop.setAttribute("data-tip", "Open a native borderless fullscreen output on a specific display. Ideal for routing HDMI to projectors or capture cards.");

    // Create dropdown container
    const dropdown = document.createElement("div");
    dropdown.id = "desktop-display-dropdown";
    dropdown.style.cssText = `
      position: fixed;
      background: var(--panel2);
      border: 1px solid var(--line);
      border-radius: 4px;
      padding: 4px;
      display: none;
      flex-direction: column;
      gap: 2px;
      z-index: 10000;
      min-width: 180px;
      box-shadow: 0 4px 12px rgba(0,0,0,0.5);
    `;
    document.body.appendChild(dropdown);
    
    // Global click to close dropdown
    document.addEventListener("click", (e) => {
      if(e.target !== btnPop && !dropdown.contains(e.target)) {
        dropdown.style.display = "none";
      }
    });

    // Hijack the click handler
    btnPop.onclick = async (e) => {
      e.stopPropagation(); // prevent immediate global close
      
      // If dropdown is already open, close it
      if (dropdown.style.display === "flex") {
        dropdown.style.display = "none";
        return;
      }
      
      const displays = await window.BendrDesktop.output.listDisplays();
      const isOpen = window.BendrDesktop.output.isOpen();
      
      dropdown.innerHTML = "";
      
      // Header
      const header = document.createElement("div");
      header.innerText = "SELECT OUTPUT DISPLAY";
      header.style.cssText = "color: var(--dim); padding: 6px 4px; font-size: 9px; text-align: center; border-bottom: 1px solid var(--line); margin-bottom: 4px; letter-spacing: 1px;";
      dropdown.appendChild(header);

      // Display options
      displays.forEach((disp, idx) => {
        const btn = document.createElement("button");
        const label = disp.is_primary ? `${disp.name} (Primary)` : disp.name;
        btn.innerText = `${label}\n${disp.width}x${disp.height}`;
        btn.style.cssText = `
          background: transparent;
          color: var(--txt);
          border: none;
          padding: 6px 8px;
          text-align: left;
          cursor: pointer;
          font-family: var(--font);
          font-size: 10px;
          line-height: 1.4;
          border-radius: 2px;
        `;
        btn.onmouseover = () => btn.style.background = "var(--line)";
        btn.onmouseout = () => btn.style.background = "transparent";
        btn.onclick = async () => {
          dropdown.style.display = "none";
          try {
            await window.BendrDesktop.output.open(idx);
            btnPop.classList.add("on");
          } catch (err) {
            console.error("Failed to open display", err);
          }
        };
        dropdown.appendChild(btn);
      });
      
      if (isOpen) {
        const closeBtn = document.createElement("button");
        closeBtn.innerText = "STOP OUTPUT";
        closeBtn.style.cssText = `
          background: rgba(255, 59, 48, 0.2);
          color: var(--red);
          border: 1px solid rgba(255, 59, 48, 0.4);
          padding: 6px 8px;
          text-align: center;
          cursor: pointer;
          font-family: var(--font);
          font-size: 10px;
          font-weight: bold;
          border-radius: 2px;
          margin-top: 4px;
        `;
        closeBtn.onmouseover = () => closeBtn.style.background = "rgba(255, 59, 48, 0.3)";
        closeBtn.onmouseout = () => closeBtn.style.background = "rgba(255, 59, 48, 0.2)";
        closeBtn.onclick = async () => {
          dropdown.style.display = "none";
          await window.BendrDesktop.output.close();
          btnPop.classList.remove("on");
        };
        dropdown.appendChild(closeBtn);
      }

      // Position the dropdown relative to the button
      const rect = btnPop.getBoundingClientRect();
      
      // If button is in the bottom half of the screen, show above it. Otherwise below.
      if (rect.top > window.innerHeight / 2) {
        dropdown.style.bottom = (window.innerHeight - rect.top + 4) + "px";
        dropdown.style.top = "auto";
      } else {
        dropdown.style.top = (rect.bottom + 4) + "px";
        dropdown.style.bottom = "auto";
      }
      
      dropdown.style.left = rect.left + "px";
      dropdown.style.display = "flex";
    };
    
    // Sync UI state if output window is closed externally
    setInterval(async () => {
      try {
        const { invoke } = window.__TAURI_INTERNALS__;
        const open = await invoke('is_output_open');
        if (!open && window.BendrDesktop.output._open) {
          window.BendrDesktop.output._open = false;
          btnPop.classList.remove("on");
        } else if (open && !window.BendrDesktop.output._open) {
          window.BendrDesktop.output._open = true;
          btnPop.classList.add("on");
        }
      } catch(e) {}
    }, 1000);
  }
}
