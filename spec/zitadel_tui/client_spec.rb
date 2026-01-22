# frozen_string_literal: true

require 'spec_helper'
require 'tty-command'

RSpec.describe ZitadelTui::Client do
  subject(:client) { described_class.new(config: config) }

  let(:config) { instance_double(ZitadelTui::Config) }
  let(:sa_key_content) { '{"keyId": "123", "userId": "user", "key": "private_key"}' }
  let(:sa_key_file) { '/tmp/zitadel-sa.json' }

  before do
    allow(config).to receive_messages(
      sa_key_file: sa_key_file,
      zitadel_url: 'https://zitadel.example.com'
    )
  end

  describe '#authenticate' do
    context 'when service account key file does not exist' do
      let(:cmd) { instance_double(TTY::Command) }
      let(:cmd_result) { instance_double(TTY::Command::Result, out: Base64.encode64(sa_key_content)) }

      before do
        allow(File).to receive(:exist?).with(sa_key_file).and_return(false)
        allow(File).to receive(:write)
        allow(File).to receive(:read).with(sa_key_file).and_return(sa_key_content)

        allow(TTY::Command).to receive(:new).and_return(cmd)
        allow(cmd).to receive(:run).and_return(cmd_result)
      end

      it 'calls kubectl via TTY::Command to get the secret' do
        # rubocop:disable RSpec/SubjectStub
        allow(client).to receive(:get_access_token).and_return('mock-token')
        # rubocop:enable RSpec/SubjectStub

        client.authenticate

        expect(cmd).to have_received(:run)
      end
    end
  end
end
