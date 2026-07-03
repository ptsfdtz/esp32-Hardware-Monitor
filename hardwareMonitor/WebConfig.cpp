#include "WebConfig.h"

#include <Arduino.h>
#include <Preferences.h>
#include <SPIFFS.h>
#include <WebServer.h>
#include <WiFi.h>

static constexpr const char *CONFIG_NAMESPACE = "monitor";
static constexpr const char *AP_SSID = "ESP32-Monitor";
static constexpr const char *DEFAULT_WIFI_SSID = "LanaoTech";
static constexpr const char *DEFAULT_WIFI_PASSWORD = "lanao2025";
static constexpr unsigned long WIFI_CONNECT_TIMEOUT = 15000;

static WebServer server(80);
static Preferences preferences;
static DisplayMode currentMode = DISPLAY_MODE_DASHBOARD;
static bool preferencesReady = false;
static bool apStarted = false;

static bool connectWiFi();
static void startSetupAccessPoint();
static void setupRoutes();
static void handleRoot();
static void handleMode();
static void handleWiFi();
static void handleStatus();
static void sendRedirect();
static String htmlPage();
static const char *modeToString(DisplayMode mode);
static DisplayMode parseMode(const String &value);

bool beginWebConfig()
{
  Serial.println("web config starting");
  if (!SPIFFS.begin(true))
  {
    Serial.println("spiffs mount failed");
  }

  preferencesReady = preferences.begin(CONFIG_NAMESPACE, false);
  if (preferencesReady)
  {
    currentMode = parseMode(preferences.getString("mode", "dashboard"));
  }

  WiFi.mode(WIFI_STA);
  WiFi.setSleep(false);

  if (!connectWiFi())
  {
    startSetupAccessPoint();
  }

  setupRoutes();
  server.begin();
  return true;
}

void handleWebConfig()
{
  server.handleClient();
}

DisplayMode getDisplayMode()
{
  return currentMode;
}

static bool connectWiFi()
{
  String ssid = DEFAULT_WIFI_SSID;
  String password = DEFAULT_WIFI_PASSWORD;

  if (preferencesReady)
  {
    ssid = preferences.getString("ssid", DEFAULT_WIFI_SSID);
    password = preferences.getString("pass", DEFAULT_WIFI_PASSWORD);
  }

  Serial.print("connecting wifi ssid=");
  Serial.println(ssid);
  WiFi.begin(ssid.c_str(), password.c_str());

  unsigned long start = millis();
  while (WiFi.status() != WL_CONNECTED && millis() - start < WIFI_CONNECT_TIMEOUT)
  {
    delay(250);
    Serial.print(".");
  }
  Serial.println();

  if (WiFi.status() == WL_CONNECTED)
  {
    Serial.print("station ip=");
    Serial.println(WiFi.localIP());
    return true;
  }

  Serial.println("wifi connect failed");
  return false;
}

static void startSetupAccessPoint()
{
  WiFi.mode(WIFI_AP_STA);

  IPAddress localIp(192, 168, 4, 1);
  IPAddress gateway(192, 168, 4, 1);
  IPAddress subnet(255, 255, 255, 0);
  WiFi.softAPConfig(localIp, gateway, subnet);

  if (WiFi.softAP(AP_SSID, nullptr, 1, false, 4))
  {
    apStarted = true;
    Serial.print("setup ap ssid=");
    Serial.println(AP_SSID);
    Serial.print("setup ap ip=");
    Serial.println(WiFi.softAPIP());
  }
  else
  {
    Serial.println("setup ap failed");
  }
}

static void setupRoutes()
{
  server.on("/", HTTP_GET, handleRoot);
  server.on("/mode", HTTP_POST, handleMode);
  server.on("/wifi", HTTP_POST, handleWiFi);
  server.on("/status", HTTP_GET, handleStatus);
}

static void handleRoot()
{
  server.send(200, "text/html; charset=utf-8", htmlPage());
}

static void handleMode()
{
  if (server.hasArg("mode"))
  {
    currentMode = parseMode(server.arg("mode"));
    if (preferencesReady)
    {
      preferences.putString("mode", modeToString(currentMode));
    }
  }

  sendRedirect();
}

static void handleWiFi()
{
  if (preferencesReady && server.hasArg("ssid"))
  {
    preferences.putString("ssid", server.arg("ssid"));
    preferences.putString("pass", server.arg("pass"));
    if (connectWiFi() && apStarted)
    {
      WiFi.softAPdisconnect(true);
      apStarted = false;
    }
  }

  sendRedirect();
}

static void handleStatus()
{
  File index = SPIFFS.open("/index.html", "r");
  bool indexExists = (bool)index;
  size_t indexSize = indexExists ? index.size() : 0;
  if (index)
  {
    index.close();
  }

  String json = "{";
  json += "\"mode\":\"";
  json += modeToString(currentMode);
  json += "\",\"ap_ip\":\"";
  json += apStarted ? WiFi.softAPIP().toString() : "";
  json += "\",\"sta_ip\":\"";
  json += WiFi.localIP().toString();
  json += "\",\"wifi_connected\":";
  json += WiFi.status() == WL_CONNECTED ? "true" : "false";
  json += ",\"index_exists\":";
  json += indexExists ? "true" : "false";
  json += ",\"index_size\":";
  json += indexSize;
  json += "}";

  server.send(200, "application/json", json);
}

static void sendRedirect()
{
  server.sendHeader("Location", "/", true);
  server.send(303, "text/plain", "");
}

static String htmlPage()
{
  String staIp = WiFi.status() == WL_CONNECTED ? WiFi.localIP().toString() : "not connected";
  String apIp = apStarted ? WiFi.softAPIP().toString() : "off";
  String mode = modeToString(currentMode);

  File file = SPIFFS.open("/index.html", "r");
  if (!file)
  {
    return "<!doctype html><meta charset=\"utf-8\"><h1>Missing /index.html</h1><p>Upload hardwareMonitor/data to SPIFFS.</p>";
  }

  if (file.size() == 0)
  {
    file.close();
    return "<!doctype html><meta charset=\"utf-8\"><h1>Empty /index.html</h1><p>Run scripts/upload-data.ps1 to upload hardwareMonitor/data.</p>";
  }

  String page = file.readString();
  file.close();

  page.replace("{{MODE}}", mode);
  page.replace("{{DASH_ACTIVE}}", currentMode == DISPLAY_MODE_DASHBOARD ? "active" : "");
  page.replace("{{SPLIT_ACTIVE}}", currentMode == DISPLAY_MODE_SPLIT ? "active" : "");
  page.replace("{{AP_IP}}", apIp);
  page.replace("{{STA_IP}}", staIp);
  return page;
}

static const char *modeToString(DisplayMode mode)
{
  return mode == DISPLAY_MODE_SPLIT ? "split" : "dashboard";
}

static DisplayMode parseMode(const String &value)
{
  return value == "split" ? DISPLAY_MODE_SPLIT : DISPLAY_MODE_DASHBOARD;
}
