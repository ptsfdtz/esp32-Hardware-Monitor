#include "Config.h"
#include "DisplayDashboard.h"
#include "Metrics.h"
#include "SerialProtocol.h"
#include "WebConfig.h"

unsigned long lastFrameTime = 0;

void setup()
{
  Serial.begin(115200);
  beginWebConfig();

  if (!beginDisplay())
  {
    Serial.println("display init failed");
  }

  lastFrameTime = millis();
}

void loop()
{
  handleWebConfig();
  readSerialData();

  unsigned long now = millis();

  if (now - lastFrameTime >= FRAME_INTERVAL)
  {
    float dt = (now - lastFrameTime) / 1000.0f;
    lastFrameTime = now;

    updateAnimation(dt);
    drawDashboard();
  }
}
