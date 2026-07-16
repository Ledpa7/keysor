import { translations } from './translations.js';
// Keysor Sandbox Interactive Simulation

document.addEventListener('DOMContentLoaded', () => {

  // Multi-language (i18n) translation dataset
  function setLanguage(lang) {
    if (!translations[lang]) lang = 'en';
    localStorage.setItem('keysor-lang', lang);
    document.documentElement.lang = lang;
    
    // Document Title & Meta Description update
    document.title = translations[lang].title;
    const metaDescription = document.querySelector('meta[name="description"]');
    if (metaDescription) {
      metaDescription.setAttribute('content', translations[lang].description);
    }

    const elements = document.querySelectorAll('[data-i18n]');
    elements.forEach(el => {
      const key = el.getAttribute('data-i18n');
      if (translations[lang][key]) {
        if (translations[lang][key].includes('<') && translations[lang][key].includes('>')) {
          el.innerHTML = translations[lang][key];
        } else {
          el.textContent = translations[lang][key];
        }
      }
    });

    // Update button active state styling
    document.querySelectorAll('.lang-btn').forEach(btn => {
      if (btn.getAttribute('data-lang') === lang) {
        btn.classList.add('text-emerald-400', 'border-emerald-500/30', 'bg-emerald-500/10');
        btn.classList.remove('text-slate-400', 'border-transparent', 'hover:text-slate-200');
      } else {
        btn.classList.remove('text-emerald-400', 'border-emerald-500/30', 'bg-emerald-500/10');
        btn.classList.add('text-slate-400', 'border-transparent', 'hover:text-slate-200');
      }
    });
  }

  // Set initial language
  const savedLang = localStorage.getItem('keysor-lang');
  const browserLang = navigator.language || navigator.userLanguage;
  let defaultLang = 'en';
  
  if (savedLang) {
    defaultLang = savedLang;
  } else if (browserLang) {
    if (browserLang.startsWith('ko')) {
      defaultLang = 'ko';
    } else if (browserLang.startsWith('zh')) {
      defaultLang = 'zh';
    }
  }
  setLanguage(defaultLang);

  // Bind language buttons
  document.querySelectorAll('.lang-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      const selectedLang = e.currentTarget.getAttribute('data-lang');
      setLanguage(selectedLang);
    });
  });

  // HUD Interactive Simulation Logic
  let hudState = {
    isPro: true,
    pixelMode: false,
    magnetMode: true,
    showDetail: false,
    sensitivity: 1.0,
    base: 1.0,
    max: 30.0,
    accel: 1.5,
    lang: defaultLang
  };

  const hudWindow = document.getElementById('hud-window');
  const hudBtnPro = null;
  const hudBtnLicense = null;
  const hudBtnLang = document.getElementById('hud-btn-lang');
  const hudSensValue = document.getElementById('hud-sens-value');
  const hudSensDetails = document.getElementById('hud-sens-details');
  const hudBtnPixel = document.getElementById('hud-btn-pixel');
  const hudBtnMagnet = document.getElementById('hud-btn-magnet');
  const hudBtnDetail = document.getElementById('hud-btn-detail');
  const hudSensDec = document.getElementById('hud-sens-dec');
  const hudSensInc = document.getElementById('hud-sens-inc');
  const hudLockOverlay = document.getElementById('hud-lock-overlay');
  const hudAlertModal = document.getElementById('hud-alert-modal');
  const hudAlertClose = document.getElementById('hud-alert-close');
  const hudBtnMin = null;
  const hudBtnClose = null;

  const hudTitleShadow = document.getElementById('hud-title-shadow');
  const hudTitleText = document.getElementById('hud-title-text');

  function updateHudUI() {
    // 1. Update Pro State styling
    if (hudState.isPro) {
      hudWindow.style.borderColor = '#2FFFAD';
      hudWindow.style.boxShadow = '0 0 30px rgba(47, 255, 173, 0.15)';
      
      // Pro Active button styling
      if (hudBtnPro) {
        hudBtnPro.setAttribute('data-i18n', 'hud_pro_active');
        hudBtnPro.className = 'h-6 px-3 bg-[#181818] border border-[#222222] text-[#2FFFAD] text-[11px] font-mono font-bold rounded cursor-pointer transition-all hover:brightness-125';
      }
      
      // Update Title Text (Keysor, Keyboard & Cursor as One!)
      hudTitleShadow.setAttribute('data-i18n', 'hud_title');
      hudTitleText.setAttribute('data-i18n', 'hud_title');
      
      // Hide Lock Overlay
      hudLockOverlay.classList.add('opacity-0', 'pointer-events-none');
      
      // Sensitivity panel border
      document.getElementById('hud-sens-panel').style.borderColor = '#2FFFAD';
    } else {
      hudWindow.style.borderColor = '#3C4040';
      hudWindow.style.boxShadow = 'none';
      
      // Buy Pro button styling
      if (hudBtnPro) {
        hudBtnPro.setAttribute('data-i18n', 'hud_buy_pro');
        hudBtnPro.className = 'h-6 px-3 bg-[#202424] border border-[#3C4040] text-[#888888] text-[11px] font-mono font-bold rounded cursor-pointer transition-all hover:bg-[#3C4040] hover:border-[#2FFFAD] hover:text-[#2FFFAD]';
      }
      
      // Update Title Text (Keysor Free...)
      hudTitleShadow.setAttribute('data-i18n', 'hud_title_free');
      hudTitleText.setAttribute('data-i18n', 'hud_title_free');
      
      // Show Lock Overlay
      hudLockOverlay.classList.remove('opacity-0', 'pointer-events-none');
      
      // Sensitivity panel border
      document.getElementById('hud-sens-panel').style.borderColor = '#3C4040';
    }

    // Translate the titles instantly in case language changed
    const currentLang = localStorage.getItem('keysor-lang') || 'en';
    const listToTranslate = [hudTitleShadow, hudTitleText];
    if (hudBtnPro) listToTranslate.push(hudBtnPro);
    listToTranslate.forEach(el => {
      const key = el.getAttribute('data-i18n');
      if (translations[currentLang] && translations[currentLang][key]) {
        el.textContent = translations[currentLang][key];
      }
    });

    // 2. Sensitivity Values
    hudSensValue.textContent = hudState.sensitivity.toFixed(1);
    document.getElementById('hud-val-base').textContent = hudState.base.toFixed(1);
    document.getElementById('hud-val-max').textContent = hudState.max.toFixed(1);
    document.getElementById('hud-val-accel').textContent = hudState.accel.toFixed(1);

    if (hudState.showDetail) {
      hudSensValue.classList.add('hidden');
      hudSensDetails.classList.remove('hidden');
      hudBtnDetail.className = 'absolute left-[20px] top-[210px] w-[100px] h-8 bg-[#2FFFAD] border border-[#3C4040] text-slate-900 text-[10px] font-bold font-mono rounded cursor-pointer transition-all hover:brightness-110';
    } else {
      hudSensValue.classList.remove('hidden');
      hudSensDetails.classList.add('hidden');
      hudBtnDetail.className = 'absolute left-[20px] top-[210px] w-[100px] h-8 bg-[#202424] border border-[#3C4040] text-slate-300 text-[10px] font-bold font-mono rounded cursor-pointer transition-all hover:border-[#2FFFAD]';
    }

    // 3. Toggles
    if (hudState.pixelMode) {
      hudBtnPixel.setAttribute('data-i18n', 'hud_pixel_on');
      hudBtnPixel.className = 'absolute left-[20px] top-[130px] w-[100px] h-8 bg-[#2FFFAD] border border-[#3C4040] text-slate-900 text-[10px] font-bold font-mono rounded cursor-pointer transition-all hover:brightness-110';
    } else {
      hudBtnPixel.setAttribute('data-i18n', 'hud_pixel_off');
      hudBtnPixel.className = 'absolute left-[20px] top-[130px] w-[100px] h-8 bg-[#202424] border border-[#3C4040] text-slate-300 text-[10px] font-bold font-mono rounded cursor-pointer transition-all hover:border-[#2FFFAD]';
    }

    if (hudState.magnetMode) {
      hudBtnMagnet.setAttribute('data-i18n', 'hud_magnet_on');
      hudBtnMagnet.className = 'absolute left-[20px] top-[170px] w-[100px] h-8 bg-[#2FFFAD] border border-[#3C4040] text-slate-900 text-[10px] font-bold font-mono rounded cursor-pointer transition-all hover:brightness-110';
    } else {
      hudBtnMagnet.setAttribute('data-i18n', 'hud_magnet_off');
      hudBtnMagnet.className = 'absolute left-[20px] top-[170px] w-[100px] h-8 bg-[#202424] border border-[#3C4040] text-slate-300 text-[10px] font-bold font-mono rounded cursor-pointer transition-all hover:border-[#2FFFAD]';
    }

    // Translate toggles
    [hudBtnPixel, hudBtnMagnet].forEach(el => {
      const key = el.getAttribute('data-i18n');
      if (translations[currentLang] && translations[currentLang][key]) {
        el.textContent = translations[currentLang][key];
      }
    });

    // Language Toggle button in HUD shows currentLang's opposite/next or toggles site lang
    hudBtnLang.textContent = currentLang.toUpperCase();
  }

  // Interactivity Handlers
  function showLockAlert() {
    hudAlertModal.classList.remove('opacity-0', 'pointer-events-none');
  }

  hudAlertClose.addEventListener('click', () => {
    hudAlertModal.classList.add('opacity-0', 'pointer-events-none');
  });

  // Toggle Pro / Free simulation
  if (hudBtnPro) {
    hudBtnPro.addEventListener('click', () => {
      hudState.isPro = !hudState.isPro;
      updateHudUI();
    });
  }

  // Top Buttons
  if (hudBtnLicense) {
    hudBtnLicense.addEventListener('click', () => {
      if (!hudState.isPro) {
        showLockAlert();
      } else {
        // Toggle back to free to show the locked status
        hudState.isPro = false;
        updateHudUI();
      }
    });
  }

  hudBtnLang.addEventListener('click', () => {
    // Toggles homepage language (EN -> KO -> ZH -> EN)
    const currentLang = localStorage.getItem('keysor-lang') || 'en';
    let nextLang = 'en';
    if (currentLang === 'en') nextLang = 'ko';
    else if (currentLang === 'ko') nextLang = 'zh';
    else nextLang = 'en';
    
    setLanguage(nextLang);
    updateHudUI();
  });

  // Minimize/Close buttons just toggle Pro or alert for fun
  if (hudBtnMin) {
    hudBtnMin.addEventListener('click', () => {
      hudWindow.style.transform = 'scale(0.95)';
      setTimeout(() => { hudWindow.style.transform = 'scale(1)'; }, 150);
    });
  }
  
  if (hudBtnClose) {
    hudBtnClose.addEventListener('click', () => {
      hudState.isPro = false;
      updateHudUI();
    });
  }

  // Sens adjust
  hudSensDec.addEventListener('click', () => {
    if (!hudState.isPro) return showLockAlert();
    hudState.sensitivity = Math.max(0.1, hudState.sensitivity - 0.1);
    hudState.base = Math.max(0.1, hudState.base - 0.1);
    updateHudUI();
  });

  hudSensInc.addEventListener('click', () => {
    if (!hudState.isPro) return showLockAlert();
    hudState.sensitivity = Math.min(9.9, hudState.sensitivity + 0.1);
    hudState.base = Math.min(9.9, hudState.base + 0.1);
    updateHudUI();
  });

  // Toggles
  hudBtnPixel.addEventListener('click', () => {
    if (!hudState.isPro) return showLockAlert();
    hudState.pixelMode = !hudState.pixelMode;
    updateHudUI();
  });

  hudBtnMagnet.addEventListener('click', () => {
    if (!hudState.isPro) return showLockAlert();
    hudState.magnetMode = !hudState.magnetMode;
    updateHudUI();
  });

  hudBtnDetail.addEventListener('click', () => {
    if (!hudState.isPro) return showLockAlert();
    hudState.showDetail = !hudState.showDetail;
    updateHudUI();
  });

  // Intercept the parent page language selection to update HUD lang button
  document.querySelectorAll('.lang-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      setTimeout(updateHudUI, 50); // slight delay to let DOM translation finish
    });
  });

  // Mobile scaling for HUD
  const hudContainer = document.getElementById('hud-container');
  const hudScaler = document.getElementById('hud-scaler');

  function resizeHud() {
    if (!hudContainer || !hudScaler) return;
    const containerWidth = hudContainer.clientWidth;
    const baseWidth = 808;
    const baseHeight = 452;

    if (containerWidth < baseWidth) {
      const scale = containerWidth / baseWidth;
      hudScaler.style.transform = `scale(${scale})`;
      hudContainer.style.height = `${baseHeight * scale}px`;
    } else {
      hudScaler.style.transform = 'none';
      hudContainer.style.height = 'auto';
    }
  }

  let resizeTimeout;
  window.addEventListener('resize', () => {
    clearTimeout(resizeTimeout);
    resizeTimeout = setTimeout(resizeHud, 100);
  });

  // Initialize
  updateHudUI();
  resizeHud();

  // Track mouse coordinates for background grid glow (throttled with rAF)
  let mouseMovePending = false;
  document.addEventListener('mousemove', (e) => {
    if (mouseMovePending) return;
    mouseMovePending = true;
    requestAnimationFrame(() => {
      document.documentElement.style.setProperty('--mouse-x', `${e.pageX}px`);
      document.documentElement.style.setProperty('--mouse-y', `${e.pageY}px`);
      mouseMovePending = false;
    });
  });

});
