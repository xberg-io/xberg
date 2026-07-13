require "./spec_helper"

describe Xberg do
  it "links the generated binding" do
    Xberg::VERSION.should_not be_empty
  end
end
