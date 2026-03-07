# frozen_string_literal: true

require 'spec_helper'

RSpec.describe ZitadelTui::Client do
  subject(:client) { described_class.new(config: config) }

  let(:config) { instance_double(ZitadelTui::Config) }

  describe '#validate_secure_url!' do
    context 'with secure URLs' do
      it 'allows https' do
        allow(config).to receive(:zitadel_url).and_return('https://zitadel.example.com')
        expect { client.send(:validate_secure_url!, URI('https://zitadel.example.com')) }.not_to raise_error
      end

      it 'allows localhost over http' do
        allow(config).to receive(:zitadel_url).and_return('http://localhost:8080')
        expect { client.send(:validate_secure_url!, URI('http://localhost:8080')) }.not_to raise_error
      end
    end

    context 'with insecure URLs' do
      it 'raises SecurityError for http' do
        allow(config).to receive(:zitadel_url).and_return('http://zitadel.example.com')
        expect do
          client.send(:validate_secure_url!,
                      URI('http://zitadel.example.com'))
        end.to raise_error(SecurityError, /Refusing to send sensitive credentials/)
      end
    end
  end
end
