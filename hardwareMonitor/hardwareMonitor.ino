#include "Config.h"
#include "DisplayDashboard.h"
#include "Metrics.h"
#include "SerialProtocol.h"

unsigned long lastFrameTime = 0;

void setup()
{
  Serial.begin(115200);

  if (!beginDisplay())
  {
    while (true)
    {
      delay(1000);
    }
  }

  lastFrameTime = millis();
}

void loop()
{
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
