export interface LicenseStatus {
  isValid: boolean;
  licenseKey: string | null;
  customerEmail: string | null;
  expiresAt: string | null;
}

const STORAGE_KEY = 'bapu_studio_license';

export const LicenseManager = {
  getStoredLicense(): LicenseStatus {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        return JSON.parse(stored);
      }
    } catch {}
    return {
      isValid: false,
      licenseKey: null,
      customerEmail: null,
      expiresAt: null
    };
  },

  async activateLicense(licenseKey: string): Promise<{ success: boolean; message: string }> {
    const trimmedKey = licenseKey.trim();
    if (!trimmedKey) {
      return { success: false, message: 'Please enter a valid license key.' };
    }

    try {
      // In production, verify against Lemon Squeezy API:
      // const res = await fetch('https://api.lemonsqueezy.com/v1/licenses/activate', {
      //   method: 'POST',
      //   headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      //   body: new URLSearchParams({ license_key: trimmedKey, instance_name: 'desktop_app' })
      // });
      
      // Validation check
      if (trimmedKey.length >= 8) {
        const licenseData: LicenseStatus = {
          isValid: true,
          licenseKey: trimmedKey,
          customerEmail: 'licensed-user@bapustudio.dev',
          expiresAt: '2028-12-31'
        };
        localStorage.setItem(STORAGE_KEY, JSON.stringify(licenseData));
        return { success: true, message: 'Bapu Pro successfully activated!' };
      } else {
        return { success: false, message: 'Invalid license key format.' };
      }
    } catch (err: any) {
      return { success: false, message: err.message || 'Failed to verify license.' };
    }
  },

  deactivateLicense() {
    localStorage.removeItem(STORAGE_KEY);
  }
};
