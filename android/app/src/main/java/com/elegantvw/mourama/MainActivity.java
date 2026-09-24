package com.elegantvw.mourama;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.SharedPreferences;
import android.os.Bundle;
import android.text.InputType;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.EditText;
import android.widget.LinearLayout;

public class MainActivity extends Activity {
    private WebView web;
    private SharedPreferences prefs;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        prefs = getSharedPreferences("mourama", MODE_PRIVATE);
        web = new WebView(this);
        setContentView(web);
        WebSettings s = web.getSettings();
        s.setJavaScriptEnabled(true);
        s.setDomStorageEnabled(true);
        s.setMixedContentMode(WebSettings.MIXED_CONTENT_ALWAYS_ALLOW);
        web.setWebViewClient(new WebViewClient());
        String url = prefs.getString("server", "");
        if (url == null || url.length() == 0) {
            askUrl("http://vanguarda:4747");
        } else {
            web.loadUrl(url);
        }
    }

    private void askUrl(String seed) {
        final EditText input = new EditText(this);
        input.setInputType(InputType.TYPE_TEXT_VARIATION_URI);
        input.setText(seed);
        LinearLayout wrap = new LinearLayout(this);
        wrap.setPadding(48, 24, 48, 8);
        wrap.addView(input);
        new AlertDialog.Builder(this)
                .setTitle("Mourama server")
                .setMessage("Office LAN or Tailscale MagicDNS")
                .setView(wrap)
                .setCancelable(false)
                .setPositiveButton("Sit down", (d, w) -> {
                    String url = input.getText().toString().trim();
                    if (!url.startsWith("http")) {
                        url = "http://" + url;
                    }
                    prefs.edit().putString("server", url).apply();
                    web.loadUrl(url);
                })
                .show();
    }

    @Override
    public void onBackPressed() {
        if (web.canGoBack()) {
            web.goBack();
        } else {
            super.onBackPressed();
        }
    }
}
